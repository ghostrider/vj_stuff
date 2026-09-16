use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use crossbeam_channel::Receiver;
use eframe::egui;
use midir::MidiInputConnection;

use crate::config::{
    self, AppConfig, InputSource, MAX_OFFSET_MS, MAX_TRIM_MS, MIN_OFFSET_MS, MIN_TRIM_MS,
    OutputSettings, RESYNC_QUANTA_BARS,
};
use crate::engine::estimator::LockState;
use crate::engine::{EngineHandle, InputEvent};
use crate::logging::LogBuffer;
use crate::output::{ClockOutput, MidirOut, OutputError, VirtualOut};
use crate::ports;
use crate::sim::SimulatorHandle;
use crate::vmidi::{VirtualPort, ffi::VmidiApi};

const SAVE_DEBOUNCE: Duration = Duration::from_millis(500);
/// ARCHITECTURE.md §7: "List all midir output ports every 2 s".
const PORT_REFRESH_INTERVAL: Duration = Duration::from_secs(2);
const INPUT_CHANNEL_CAPACITY: usize = 1024;
/// We only care about realtime bytes (clock/start/continue/stop), never Sysex, so
/// there is no point asking the driver to reassemble one for us.
const MAX_SYSEX_LENGTH: u32 = 0;

/// Whichever concrete resource is currently feeding the engine, kept alive here
/// so dropping/replacing it (input source switching, ARCHITECTURE.md §3) closes
/// the old one before opening the new one.
enum ActiveInput {
    None,
    /// The virtual `Clock Shifter In` port itself lives in `App::input_port` and
    /// keeps running regardless of selection; this variant just marks that the
    /// engine is currently subscribed to it.
    Virtual,
    /// Kept only to hold the connection open; never read, closed on `Drop`.
    Hardware {
        _connection: MidiInputConnection<()>,
    },
    /// Kept only to keep the simulator thread running; closed on `Drop`.
    Simulator {
        handle: SimulatorHandle,
    },
}

pub struct App {
    config: AppConfig,
    config_path: Option<PathBuf>,
    dirty: bool,
    last_edit: Instant,
    last_port_refresh: Instant,
    input_ports: Vec<String>,
    output_ports: Vec<String>,
    /// All `midir` input ports, unfiltered — Measure panel port list
    /// (PROMPTS.md Prompt 7; see [`ports::list_all_input_ports`]).
    measure_ports: Vec<String>,
    /// Kept only to hold the Measure panel's input connection open; never
    /// read directly (the engine consumes its channel), closed on `Drop`.
    measure_connection: Option<MidiInputConnection<()>>,
    vmidi_error: Option<String>,
    vmidi_dll_version: Option<String>,
    vmidi_driver_version: Option<String>,
    /// Kept around to construct `VirtualOut` on demand whenever the virtual
    /// output is (re-)enabled; `None` if the driver/DLL is unavailable.
    vmidi_api: Option<Arc<VmidiApi>>,
    input_port: Option<VirtualPort>,
    input_rx: Receiver<InputEvent>,
    active_input: ActiveInput,
    engine: EngineHandle,
    log_buffer: Arc<LogBuffer>,
}

impl App {
    pub fn new(
        config: AppConfig,
        config_path: Option<PathBuf>,
        log_buffer: Arc<LogBuffer>,
    ) -> Self {
        let (input_tx, input_rx) = crossbeam_channel::bounded(INPUT_CHANNEL_CAPACITY);

        let mut vmidi_error = None;
        let mut vmidi_dll_version = None;
        let mut vmidi_driver_version = None;
        let mut vmidi_api = None;
        let mut input_port = None;

        match VmidiApi::load() {
            Ok(api) => {
                let api = Arc::new(api);
                vmidi_dll_version = api.version_string();
                vmidi_driver_version = api.driver_version_string();

                match VirtualPort::create_input(
                    Arc::clone(&api),
                    &config.virtual_in_name,
                    MAX_SYSEX_LENGTH,
                    input_tx,
                ) {
                    Ok(port) => {
                        log::info!("created virtual input port '{}'", config.virtual_in_name);
                        input_port = Some(port);
                    }
                    Err(err) => {
                        log::warn!(
                            "failed to create virtual input port '{}': {err}",
                            config.virtual_in_name
                        );
                        vmidi_error.get_or_insert_with(|| err.to_string());
                    }
                }

                vmidi_api = Some(api);
            }
            Err(err) => {
                log::warn!("virtualMIDI unavailable: {err}");
                vmidi_error = Some(err.to_string());
            }
        }

        let mut app = Self {
            config,
            config_path,
            dirty: false,
            last_edit: Instant::now(),
            // Force an immediate refresh on the first frame.
            last_port_refresh: Instant::now() - PORT_REFRESH_INTERVAL,
            input_ports: Vec::new(),
            output_ports: Vec::new(),
            measure_ports: Vec::new(),
            measure_connection: None,
            vmidi_error,
            vmidi_dll_version,
            vmidi_driver_version,
            vmidi_api,
            input_port,
            input_rx,
            active_input: ActiveInput::None,
            engine: EngineHandle::start(),
            log_buffer,
        };
        app.refresh_ports();
        app.apply_input_source();
        app.engine
            .set_global_offset_ms(app.config.global_offset_ms as f64);
        app.engine
            .set_resync_quantum_bars(app.config.resync_quantum_bars as u32);
        app.engine.set_forward_continue(app.config.forward_continue);
        app.apply_outputs();
        app.apply_measure_input();
        app
    }

    /// (Re-)applies `self.config.input_source`: closes whatever the engine was
    /// previously reading from and opens the newly-selected one, resetting the
    /// estimator (ARCHITECTURE.md: "Input source switching ... close old, open
    /// new, estimator reset").
    fn apply_input_source(&mut self) {
        // Drop the old resource (closes the hardware connection / stops the
        // simulator thread) before opening the new one.
        self.active_input = ActiveInput::None;

        match self.config.input_source.clone() {
            InputSource::VirtualIn => {
                self.engine.set_input(self.input_rx.clone());
                self.active_input = ActiveInput::Virtual;
            }
            InputSource::Hardware(name) => {
                let (tx, rx) = crossbeam_channel::bounded(INPUT_CHANNEL_CAPACITY);
                match ports::open_hardware_input(&name, tx) {
                    Ok(connection) => {
                        self.engine.set_input(rx);
                        self.active_input = ActiveInput::Hardware {
                            _connection: connection,
                        };
                    }
                    Err(err) => {
                        log::warn!("failed to open hardware input '{name}': {err}");
                        self.engine.clear_input();
                    }
                }
            }
            InputSource::Simulator => {
                let (tx, rx) = crossbeam_channel::bounded(INPUT_CHANNEL_CAPACITY);
                let handle = SimulatorHandle::spawn(
                    self.config.simulator.bpm,
                    self.config.simulator.jitter_ms,
                    tx,
                );
                self.engine.set_input(rx);
                self.active_input = ActiveInput::Simulator { handle };
            }
        }
    }

    /// (Re-)applies `self.config.measure_input` (PROMPTS.md Prompt 7): closes
    /// the previous measure connection (if any) and opens the newly-selected
    /// one, resetting the engine's running lead stats — mirrors
    /// `apply_input_source`'s "close old, open new" pattern, but for the
    /// independent Measure panel input rather than the main clock pipeline.
    fn apply_measure_input(&mut self) {
        self.measure_connection = None;

        let Some(name) = self.config.measure_input.clone() else {
            self.engine.clear_measure_input();
            return;
        };

        let (tx, rx) = crossbeam_channel::bounded(INPUT_CHANNEL_CAPACITY);
        match ports::open_hardware_input(&name, tx) {
            Ok(connection) => {
                log::info!("measure input '{name}' connected");
                self.engine.set_measure_input(rx);
                self.measure_connection = Some(connection);
            }
            Err(err) => {
                log::warn!("failed to open measure input '{name}': {err}");
                self.engine.clear_measure_input();
            }
        }
    }

    /// Closes the current `Clock Shifter In` virtual port (if any) and opens a
    /// fresh one under `self.config.virtual_in_name` (ARCHITECTURE.md §7:
    /// "Changing virtual port names in settings recreates the ports safely").
    /// A new channel pair is used rather than reusing the old sender, since
    /// `VirtualPort::create_input` binds a sender at construction time.
    fn recreate_virtual_input(&mut self) {
        // Drop the old port first, closing it, before opening the new one
        // under the new name (same "close old, then open new" ordering as
        // `apply_input_source`).
        self.input_port = None;

        let (tx, rx) = crossbeam_channel::bounded(INPUT_CHANNEL_CAPACITY);
        if let Some(api) = self.vmidi_api.clone() {
            match VirtualPort::create_input(api, &self.config.virtual_in_name, MAX_SYSEX_LENGTH, tx)
            {
                Ok(port) => {
                    log::info!(
                        "recreated virtual input port '{}'",
                        self.config.virtual_in_name
                    );
                    self.input_port = Some(port);
                }
                Err(err) => {
                    log::warn!(
                        "failed to recreate virtual input port '{}': {err}",
                        self.config.virtual_in_name
                    );
                }
            }
        }
        self.input_rx = rx;

        if matches!(self.config.input_source, InputSource::VirtualIn) {
            self.apply_input_source();
        }
    }

    /// Moves the `Clock Shifter Out` connection (and its settings, so trim
    /// and enabled/forward-transport state survive) from `old_name` to
    /// `self.config.virtual_out_name`, and reconnects under the new name if
    /// it was enabled (ARCHITECTURE.md §7: "recreates the ports safely").
    fn rename_virtual_output(&mut self, old_name: &str) {
        self.engine.remove_output(old_name.to_string());
        if let Some(settings) = self.config.outputs.remove(old_name) {
            self.config
                .outputs
                .insert(self.config.virtual_out_name.clone(), settings);
        }
        log::info!(
            "renamed virtual output port '{old_name}' -> '{}'",
            self.config.virtual_out_name
        );
        self.apply_output_connection(&self.config.virtual_out_name.clone());
    }

    fn refresh_ports(&mut self) {
        // ARCHITECTURE.md §7 "Loop protection" excludes each virtual port from
        // the *opposite* list; also drop our own virtual ports from their
        // *own* list so they don't show up a second time next to the
        // hardcoded first row that already represents them.
        self.input_ports = ports::list_input_ports(&self.config.virtual_out_name)
            .into_iter()
            .filter(|name| name != &self.config.virtual_in_name)
            .collect();
        self.output_ports = ports::list_output_ports(&self.config.virtual_in_name)
            .into_iter()
            .filter(|name| name != &self.config.virtual_out_name)
            .collect();
        self.measure_ports = ports::list_all_input_ports();
        self.config
            .outputs
            .entry(self.config.virtual_out_name.clone())
            .or_default();
        for name in &self.output_ports {
            self.config.outputs.entry(name.clone()).or_default();
        }
        self.last_port_refresh = Instant::now();
        self.reconcile_ports();
    }

    /// Retries a hardware output/input connection that previously failed or
    /// dropped, once its port reappears in the live enumeration
    /// (ARCHITECTURE.md §7: "It reappears -> reconnect automatically, keep
    /// settings, log it"). Runs on the same ~1s cadence as `refresh_ports`
    /// (this app polls from the UI thread rather than a separate watcher
    /// thread — see ARCHITECTURE.md §3 for the deviation note).
    fn reconcile_ports(&mut self) {
        let snapshot = self.engine.snapshot();
        let connected: std::collections::HashSet<&str> = snapshot
            .outputs
            .iter()
            .filter(|status| status.connected)
            .map(|status| status.name.as_str())
            .collect();

        let to_reconnect: Vec<String> = self
            .config
            .outputs
            .iter()
            .filter(|(name, settings)| {
                settings.enabled
                    && name.as_str() != self.config.virtual_out_name
                    && self.output_ports.iter().any(|p| p == name.as_str())
                    && !connected.contains(name.as_str())
            })
            .map(|(name, _)| name.clone())
            .collect();
        for name in to_reconnect {
            log::info!("output '{name}' is available again, reconnecting");
            self.apply_output_connection(&name);
        }

        if let InputSource::Hardware(name) = self.config.input_source.clone()
            && matches!(self.active_input, ActiveInput::None)
            && self.input_ports.iter().any(|p| p == &name)
        {
            log::info!("input '{name}' is available again, reconnecting");
            self.apply_input_source();
        }

        if let Some(name) = self.config.measure_input.clone()
            && self.measure_connection.is_none()
            && self.measure_ports.iter().any(|p| p == &name)
        {
            log::info!("measure input '{name}' is available again, reconnecting");
            self.apply_measure_input();
        }
    }

    /// Constructs the concrete [`ClockOutput`] for `name` (the virtual port if
    /// it matches the configured virtual output name, a hardware port
    /// otherwise) — the one place that knows which is which.
    fn create_output(&self, name: &str) -> Result<Box<dyn ClockOutput>, OutputError> {
        if name == self.config.virtual_out_name {
            let api = self
                .vmidi_api
                .clone()
                .ok_or_else(|| OutputError::Init("virtualMIDI unavailable".to_string()))?;
            Ok(Box::new(VirtualOut::create(api, name)?))
        } else {
            Ok(Box::new(MidirOut::open(name)?))
        }
    }

    /// Connects (if `enabled`) or disconnects (if not) the named output in the
    /// engine, matching `self.config.outputs[name].enabled`
    /// (ARCHITECTURE.md: "Move all output connections into the engine thread").
    fn apply_output_connection(&mut self, name: &str) {
        let Some(settings) = self.config.outputs.get(name) else {
            return;
        };
        if settings.enabled {
            let trim_ms = settings.trim_ms as f64;
            let forward_transport = settings.forward_transport;
            match self.create_output(name) {
                Ok(output) => {
                    log::info!("output '{name}' connected");
                    self.engine
                        .add_output(name.to_string(), output, trim_ms, forward_transport)
                }
                Err(err) => log::warn!("failed to enable output '{name}': {err}"),
            }
        } else {
            self.engine.remove_output(name.to_string());
        }
    }

    /// Connects every output marked `enabled` in the config (called once at
    /// startup; live toggles go through [`Self::apply_output_connection`]).
    fn apply_outputs(&mut self) {
        let enabled_names: Vec<String> = self
            .config
            .outputs
            .iter()
            .filter(|(_, settings)| settings.enabled)
            .map(|(name, _)| name.clone())
            .collect();
        for name in enabled_names {
            self.apply_output_connection(&name);
        }
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
        self.last_edit = Instant::now();
    }

    fn maybe_save(&mut self) {
        if self.dirty && self.last_edit.elapsed() >= SAVE_DEBOUNCE {
            if let Err(err) = self.config.save(self.config_path.as_deref()) {
                log::warn!("failed to save config: {err}");
            }
            self.dirty = false;
        }
    }

    fn toolbar_ui(&mut self, ui: &mut egui::Ui) {
        let mut changed = false;
        let snapshot = self.engine.snapshot();

        ui.horizontal_wrapped(|ui| {
            ui.label("Offset");
            let mut offset_changed = false;
            let mut offset = self.config.global_offset_ms;
            if ui
                .add(egui::Slider::new(&mut offset, MIN_OFFSET_MS..=MAX_OFFSET_MS).suffix(" ms"))
                .changed()
            {
                self.config.global_offset_ms = config::clamp_offset_ms(offset);
                offset_changed = true;
            }
            let mut offset_exact = self.config.global_offset_ms;
            if ui
                .add(egui::DragValue::new(&mut offset_exact).suffix(" ms"))
                .changed()
            {
                self.config.global_offset_ms = config::clamp_offset_ms(offset_exact);
                offset_changed = true;
            }
            if offset_changed {
                self.engine
                    .set_global_offset_ms(self.config.global_offset_ms as f64);
                changed = true;
            }

            let (bpm_for_beats, beats_note) = if snapshot.bpm > 0.0 {
                (snapshot.bpm, "")
            } else {
                (
                    self.config.simulator.bpm,
                    " (using sim. BPM, no live clock)",
                )
            };
            let beats = self.config.global_offset_ms as f64 / 60_000.0 * bpm_for_beats;
            ui.label(format!("= {beats:.2} beats{beats_note}"));

            ui.separator();

            ui.label("Resync quantum");
            let mut quantum_changed = false;
            egui::ComboBox::from_id_salt("resync_quantum")
                .selected_text(format!("{} bar(s)", self.config.resync_quantum_bars))
                .show_ui(ui, |ui| {
                    for quantum in RESYNC_QUANTA_BARS {
                        if ui
                            .selectable_value(
                                &mut self.config.resync_quantum_bars,
                                quantum,
                                format!("{quantum} bar(s)"),
                            )
                            .changed()
                        {
                            quantum_changed = true;
                        }
                    }
                });
            if quantum_changed {
                self.engine
                    .set_resync_quantum_bars(self.config.resync_quantum_bars as u32);
                changed = true;
            }

            ui.separator();

            let resync_label = if snapshot.pending_start {
                "Resync (pending…)"
            } else {
                "Resync"
            };
            if ui
                .add(egui::Button::new(resync_label))
                .on_hover_text(
                    "Sends a manual 0xFA (start) to all enabled outputs with \"Fwd start/stop\" on",
                )
                .clicked()
            {
                self.engine.manual_resync();
            }

            ui.separator();

            if let Some(pos) = snapshot.output_position {
                let beat = pos.tick_in_bar / 24;
                let flashing = beat == 0 && pos.bar_in_phrase == 0 && pos.tick_in_bar < 4;
                ui.horizontal(|ui| {
                    for b in 0..4u32 {
                        let color = if b == beat {
                            if flashing && b == 0 {
                                egui::Color32::WHITE
                            } else {
                                egui::Color32::from_rgb(120, 200, 120)
                            }
                        } else {
                            egui::Color32::DARK_GRAY
                        };
                        ui.colored_label(color, "●");
                    }
                    ui.colored_label(
                        egui::Color32::GRAY,
                        format!(
                            "bar {}/{}",
                            pos.bar_in_phrase + 1,
                            self.config.resync_quantum_bars
                        ),
                    );
                });
                ui.separator();
            }

            let (lock_text, lock_color) = match snapshot.lock_state {
                LockState::NoSignal => ("NoSignal", egui::Color32::GRAY),
                LockState::Acquiring => ("Acquiring", egui::Color32::from_rgb(220, 180, 40)),
                LockState::Locked => ("Locked", egui::Color32::from_rgb(120, 200, 120)),
            };
            ui.colored_label(lock_color, format!("Lock: {lock_text}"));
            ui.colored_label(egui::Color32::GRAY, format!("BPM in: {:.2}", snapshot.bpm));
            ui.colored_label(
                egui::Color32::GRAY,
                format!("Input jitter: {:.2} ms", snapshot.input_jitter_ms),
            );
            ui.colored_label(
                egui::Color32::GRAY,
                format!("Send jitter (p99): {:.2} ms", snapshot.send_jitter_p99_ms),
            );
        });

        if let Some(err) = &self.vmidi_error {
            ui.colored_label(
                egui::Color32::from_rgb(220, 80, 80),
                format!("virtualMIDI unavailable: {err}"),
            );
        } else {
            ui.colored_label(
                egui::Color32::GRAY,
                format!(
                    "virtualMIDI dll {} / driver {}",
                    self.vmidi_dll_version.as_deref().unwrap_or("unknown"),
                    self.vmidi_driver_version.as_deref().unwrap_or("unknown"),
                ),
            );
        }

        if changed {
            self.mark_dirty();
        }
    }

    fn input_panel_ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Input source");
        let mut changed = false;

        let virtual_status = match &self.input_port {
            Some(port) if port.is_open() => "connected",
            Some(_) => "closed by driver",
            None => "unavailable",
        };
        let virtual_label = format!(
            "{} (virtual, default) — {virtual_status}",
            self.config.virtual_in_name
        );
        if ui
            .radio_value(
                &mut self.config.input_source,
                InputSource::VirtualIn,
                virtual_label,
            )
            .changed()
        {
            changed = true;
        }

        // Keep showing the currently-selected hardware input even if it just
        // dropped out of the live enumeration, so unplugging it doesn't make
        // the selection silently vanish (ARCHITECTURE.md §7/§9: "Missing
        // devices shown grey"); it reappears/reconnects automatically in
        // `reconcile_ports` once the device is back.
        let mut hardware_names = self.input_ports.clone();
        if let InputSource::Hardware(selected) = &self.config.input_source
            && !hardware_names.iter().any(|n| n == selected)
        {
            hardware_names.push(selected.clone());
        }

        for name in &hardware_names {
            let present = self.input_ports.iter().any(|p| p == name);
            let label = if present {
                name.clone()
            } else {
                format!("{name} (missing)")
            };
            ui.add_enabled_ui(present, |ui| {
                if ui
                    .radio_value(
                        &mut self.config.input_source,
                        InputSource::Hardware(name.clone()),
                        label,
                    )
                    .changed()
                {
                    changed = true;
                }
            });
        }

        if ui
            .radio_value(
                &mut self.config.input_source,
                InputSource::Simulator,
                "Simulator",
            )
            .changed()
        {
            changed = true;
        }

        if changed {
            self.apply_input_source();
            self.mark_dirty();
        }
    }

    fn output_panel_ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Outputs");

        // Row source is every name the config has ever seen (always includes
        // everything currently live, via `refresh_ports`'s `entry(..).or_default()`),
        // not just the current live enumeration — so a previously-enabled
        // output that just went missing keeps its row instead of vanishing
        // (ARCHITECTURE.md §7: "Missing devices shown grey").
        let mut names = vec![self.config.virtual_out_name.clone()];
        names.extend(
            self.config
                .outputs
                .keys()
                .filter(|name| name.as_str() != self.config.virtual_out_name)
                .cloned(),
        );

        let snapshot = self.engine.snapshot();
        let connected_by_name: HashMap<&str, bool> = snapshot
            .outputs
            .iter()
            .map(|status| (status.name.as_str(), status.connected))
            .collect();

        let mut changed = false;
        let mut connection_changes = Vec::new();
        let mut trim_changes = Vec::new();
        let mut forward_transport_changes = Vec::new();
        egui::Grid::new("outputs_grid")
            .striped(true)
            .num_columns(5)
            .show(ui, |ui| {
                ui.label("On");
                ui.label("Name");
                ui.label("Status");
                ui.label("Trim");
                ui.label("Fwd start/stop");
                ui.end_row();

                for name in &names {
                    let enabled = self
                        .config
                        .outputs
                        .get(name.as_str())
                        .is_some_and(|s| s.enabled);
                    let present = if name == &self.config.virtual_out_name {
                        self.vmidi_api.is_some()
                    } else {
                        self.output_ports.iter().any(|p| p == name.as_str())
                    };
                    let (status, color) = if !enabled {
                        ("disabled", egui::Color32::GRAY)
                    } else if !present {
                        ("missing", egui::Color32::GRAY)
                    } else if connected_by_name
                        .get(name.as_str())
                        .copied()
                        .unwrap_or(false)
                    {
                        ("connected", egui::Color32::from_rgb(120, 200, 120))
                    } else {
                        ("not connected", egui::Color32::from_rgb(220, 80, 80))
                    };

                    let row_change = output_row(ui, &mut self.config.outputs, name, status, color);
                    if row_change.enabled_toggled {
                        connection_changes.push(name.clone());
                        changed = true;
                    }
                    if row_change.trim_changed {
                        trim_changes.push(name.clone());
                        changed = true;
                    }
                    if row_change.forward_transport_toggled {
                        forward_transport_changes.push(name.clone());
                        changed = true;
                    }
                }
            });

        for name in connection_changes {
            self.apply_output_connection(&name);
        }
        for name in trim_changes {
            if let Some(settings) = self.config.outputs.get(&name)
                && settings.enabled
            {
                self.engine
                    .set_output_trim_ms(name.clone(), settings.trim_ms as f64);
            }
        }
        for name in forward_transport_changes {
            if let Some(settings) = self.config.outputs.get(&name)
                && settings.enabled
            {
                self.engine
                    .set_output_forward_transport(name.clone(), settings.forward_transport);
            }
        }

        if changed {
            self.mark_dirty();
        }
    }

    /// Temporary raw message counters for the virtual input port (Prompt 2); the
    /// timing/interval display it originally had is superseded by the real
    /// lock-state/BPM/jitter readout in the toolbar (Prompt 3).
    fn debug_ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("virtualMIDI debug (temporary)");

        if let Some(port) = &self.input_port {
            let counters = &port.counters;
            let clock = counters.clock.load(Ordering::Relaxed);
            let start = counters.start.load(Ordering::Relaxed);
            let cont = counters.continue_msgs.load(Ordering::Relaxed);
            let stop = counters.stop.load(Ordering::Relaxed);
            let overflow = counters.overflow.load(Ordering::Relaxed);

            ui.horizontal_wrapped(|ui| {
                ui.label(format!("Clock: {clock}"));
                ui.label(format!("Start: {start}"));
                ui.label(format!("Continue: {cont}"));
                ui.label(format!("Stop: {stop}"));
                if overflow > 0 {
                    ui.colored_label(
                        egui::Color32::from_rgb(220, 150, 40),
                        format!("Overflow: {overflow}"),
                    );
                }
            });

            if !port.is_open() {
                ui.colored_label(
                    egui::Color32::from_rgb(220, 80, 80),
                    "Clock Shifter In: driver reported the port closed",
                );
            }
        } else {
            ui.colored_label(egui::Color32::GRAY, "Clock Shifter In not available.");
        }
    }

    fn settings_ui(&mut self, ui: &mut egui::Ui) {
        let mut changed = false;
        let mut sim_params_changed = false;
        egui::CollapsingHeader::new("Settings").show(ui, |ui| {
            egui::Grid::new("settings_grid")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Virtual input port name");
                    let old_virtual_in_name = self.config.virtual_in_name.clone();
                    let response = ui.text_edit_singleline(&mut self.config.virtual_in_name);
                    if response.changed() {
                        changed = true;
                    }
                    // Recreate only once editing finishes (blur/Enter), not on
                    // every keystroke, so Pulse/Resolume don't see the port
                    // flicker closed/reopened while the name is half-typed.
                    if response.lost_focus() && self.config.virtual_in_name != old_virtual_in_name {
                        self.recreate_virtual_input();
                    }
                    ui.end_row();

                    ui.label("Virtual output port name");
                    let old_virtual_out_name = self.config.virtual_out_name.clone();
                    let response = ui.text_edit_singleline(&mut self.config.virtual_out_name);
                    if response.changed() {
                        changed = true;
                    }
                    if response.lost_focus() && self.config.virtual_out_name != old_virtual_out_name
                    {
                        self.rename_virtual_output(&old_virtual_out_name);
                    }
                    ui.end_row();

                    ui.label("Forward 0xFB Continue");
                    if ui.checkbox(&mut self.config.forward_continue, "").changed() {
                        self.engine
                            .set_forward_continue(self.config.forward_continue);
                        changed = true;
                    }
                    ui.end_row();

                    ui.label("Simulator BPM");
                    if ui
                        .add(
                            egui::DragValue::new(&mut self.config.simulator.bpm)
                                .range(30.0..=300.0)
                                .suffix(" BPM"),
                        )
                        .changed()
                    {
                        changed = true;
                        sim_params_changed = true;
                    }
                    ui.end_row();

                    ui.label("Simulator jitter");
                    if ui
                        .add(
                            egui::DragValue::new(&mut self.config.simulator.jitter_ms)
                                .range(0.0..=20.0)
                                .suffix(" ms"),
                        )
                        .changed()
                    {
                        changed = true;
                        sim_params_changed = true;
                    }
                    ui.end_row();
                });
        });

        if sim_params_changed && let ActiveInput::Simulator { handle } = &self.active_input {
            handle.set_params(self.config.simulator.bpm, self.config.simulator.jitter_ms);
        }

        if changed {
            self.mark_dirty();
        }
    }

    /// Measure panel (PROMPTS.md Prompt 7): pick any `midir` input port —
    /// typically `Clock Shifter Out` looped straight back, or a hardware port
    /// physically looped to an enabled output — and show the real-world lead
    /// measured on it, relative to the main input's predicted tick times.
    /// This is the "real timing validation" home the timing-and-jitter skill
    /// reserves for live measurement rather than `cargo test`.
    fn measure_ui(&mut self, ui: &mut egui::Ui) {
        let mut changed = false;
        egui::CollapsingHeader::new("Measure").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Measure input");
                let selected_text = self
                    .config
                    .measure_input
                    .clone()
                    .unwrap_or_else(|| "(none)".to_string());
                egui::ComboBox::from_id_salt("measure_input")
                    .selected_text(selected_text)
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_value(&mut self.config.measure_input, None, "(none)")
                            .changed()
                        {
                            changed = true;
                        }
                        for name in self.measure_ports.clone() {
                            if ui
                                .selectable_value(
                                    &mut self.config.measure_input,
                                    Some(name.clone()),
                                    name,
                                )
                                .changed()
                            {
                                changed = true;
                            }
                        }
                    });
            });

            if self.config.measure_input.is_some() {
                let status = if self.measure_connection.is_some() {
                    ("connected", egui::Color32::from_rgb(120, 200, 120))
                } else {
                    ("not connected", egui::Color32::from_rgb(220, 80, 80))
                };
                let snapshot = self.engine.snapshot();
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(status.1, status.0);
                    ui.separator();
                    ui.label(format!(
                        "Measured lead: mean {:.2} ms, p99 {:.2} ms",
                        snapshot.measure_lead_mean_ms, snapshot.measure_lead_p99_ms
                    ));
                });
            } else {
                ui.colored_label(
                    egui::Color32::GRAY,
                    "Select a port (e.g. Clock Shifter Out looped back, or a hardware port \
                     looped to an enabled output) to measure real-world lead.",
                );
            }
        });

        if changed {
            self.apply_measure_input();
            self.mark_dirty();
        }
    }

    fn log_ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Log");
        let lines = self.log_buffer.snapshot();
        egui::ScrollArea::vertical()
            .id_salt("log_scroll")
            .max_height(200.0)
            .stick_to_bottom(true)
            .show(ui, |ui| {
                for line in &lines {
                    let secs = line.elapsed.as_secs_f64();
                    let color = match line.level {
                        log::Level::Error => egui::Color32::from_rgb(220, 80, 80),
                        log::Level::Warn => egui::Color32::from_rgb(220, 180, 40),
                        _ => egui::Color32::GRAY,
                    };
                    ui.horizontal(|ui| {
                        ui.colored_label(egui::Color32::DARK_GRAY, format!("[{secs:>8.3}s]"));
                        ui.colored_label(color, format!("{:<5}", line.level));
                        ui.label(&line.message);
                    });
                }
                if lines.is_empty() {
                    ui.colored_label(egui::Color32::GRAY, "(no log lines yet)");
                }
            });
    }
}

/// What changed in one call to [`output_row`], so the caller knows which
/// engine commands (if any) to send.
#[derive(Debug, Default)]
struct OutputRowChange {
    enabled_toggled: bool,
    trim_changed: bool,
    forward_transport_toggled: bool,
}

fn output_row(
    ui: &mut egui::Ui,
    outputs: &mut BTreeMap<String, OutputSettings>,
    name: &str,
    status: &str,
    status_color: egui::Color32,
) -> OutputRowChange {
    let settings = outputs.entry(name.to_string()).or_default();
    let mut change = OutputRowChange::default();

    if ui.checkbox(&mut settings.enabled, "").changed() {
        change.enabled_toggled = true;
    }

    ui.label(name);
    ui.colored_label(status_color, status);

    let mut trim = settings.trim_ms;
    if ui
        .add(egui::Slider::new(&mut trim, MIN_TRIM_MS..=MAX_TRIM_MS).suffix(" ms"))
        .changed()
    {
        settings.trim_ms = config::clamp_trim_ms(trim);
        change.trim_changed = true;
    }

    if ui.checkbox(&mut settings.forward_transport, "").changed() {
        change.forward_transport_toggled = true;
    }

    ui.end_row();
    change
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if self.last_port_refresh.elapsed() >= PORT_REFRESH_INTERVAL {
            self.refresh_ports();
        }

        egui::Panel::top("toolbar").show(ui, |ui| {
            self.toolbar_ui(ui);
        });

        egui::Panel::left("input_panel")
            .resizable(true)
            .show(ui, |ui| {
                self.input_panel_ui(ui);
            });

        egui::CentralPanel::default().show(ui, |ui| {
            self.output_panel_ui(ui);
            ui.separator();
            self.debug_ui(ui);
            ui.separator();
            self.settings_ui(ui);
            ui.separator();
            self.measure_ui(ui);
            ui.separator();
            self.log_ui(ui);
        });

        self.maybe_save();
        ui.ctx().request_repaint_after(Duration::from_millis(33));
    }

    fn on_exit(&mut self) {
        log::info!("shutting down");
        if let Err(err) = self.config.save(self.config_path.as_deref()) {
            log::warn!("failed to save config on exit: {err}");
        }
        // The rest of shutdown happens via `Drop`, in field declaration order,
        // once eframe drops this `App` after `on_exit` returns: `input_port`
        // closes the virtual input, `active_input` closes the hardware
        // connection or stops the simulator thread, and `engine` sends
        // `Shutdown` and joins the engine thread — which drops every
        // `OutputEntry`, closing all output ports — before its `Drop` returns.
        // `main.rs`'s timer-resolution guard restores timer resolution once
        // `eframe::run_native` returns (ARCHITECTURE.md §12: "graceful shutdown").
    }
}
