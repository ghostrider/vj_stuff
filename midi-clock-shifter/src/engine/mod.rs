pub mod estimator;
pub mod scheduler;
pub mod transport;

use std::collections::HashMap;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use arc_swap::ArcSwap;
use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, TryRecvError};

use crate::clock::TimeSource;
use crate::output::ClockOutput;

/// Real-time MIDI status bytes this app cares about (ARCHITECTURE.md §5.1).
pub const CLOCK: u8 = 0xF8;
pub const START: u8 = 0xFA;
pub const CONTINUE: u8 = 0xFB;
pub const STOP: u8 = 0xFC;

/// A real-time MIDI message relevant to this app, mapped from its status byte.
/// `Copy` so it can be timestamped and queued without allocation on the hot path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RtMsg {
    Clock,
    Start,
    Continue,
    Stop,
}

impl RtMsg {
    /// Maps a real-time status byte to an [`RtMsg`], or `None` for anything else
    /// (including 0xFE active sensing, which is never treated as clock).
    pub fn from_status_byte(byte: u8) -> Option<Self> {
        match byte {
            CLOCK => Some(Self::Clock),
            START => Some(Self::Start),
            CONTINUE => Some(Self::Continue),
            STOP => Some(Self::Stop),
            _ => None,
        }
    }
}

/// A timestamped [`RtMsg`], produced by an input callback and queued for the engine
/// thread to drain (ARCHITECTURE.md §3 Threads). `Copy` so queueing never allocates.
#[derive(Debug, Clone, Copy)]
pub struct InputEvent {
    pub at: Instant,
    pub msg: RtMsg,
}

/// Per-output connection status, part of the published [`Snapshot`].
#[derive(Debug, Clone)]
pub struct OutputStatus {
    pub name: String,
    pub connected: bool,
}

/// The **output** (shifted) transport position, for the UI's beat display
/// (ARCHITECTURE.md §9: "beat cells + bar/phrase counter of the output
/// position, beat 1 flashes"). `None` until a start has established a
/// position counter (ARCHITECTURE.md §5.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutputPosition {
    pub tick_in_bar: u32,
    pub bar_in_phrase: u32,
}

/// Published by the engine thread for the UI to read (ARCHITECTURE.md §3: "Engine
/// -> UI: `arc-swap`").
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    pub lock_state: estimator::LockState,
    pub bpm: f64,
    pub input_jitter_ms: f64,
    /// 99th percentile of recent |scheduled - actual| output send errors, in ms
    /// (ARCHITECTURE.md §8: "measures this and shows it ... 'send jitter'").
    pub send_jitter_p99_ms: f64,
    pub outputs: Vec<OutputStatus>,
    /// A start is pending (armed on at least one output, or waiting for the
    /// estimator to lock) — ARCHITECTURE.md §9: "pending-start indicator".
    pub pending_start: bool,
    pub output_position: Option<OutputPosition>,
    /// Measure panel (PROMPTS.md Prompt 7): mean/p99 of the real-world lead
    /// measured on a separately-selected MIDI input, relative to the main
    /// input's predicted tick times. `0.0` (both) when no measure input is
    /// selected or no samples have arrived yet.
    pub measure_lead_mean_ms: f64,
    pub measure_lead_p99_ms: f64,
}

/// Commands sent from the UI thread to the engine thread.
enum Command {
    /// Switch the active input source: drain from `rx` from now on, and reset
    /// the estimator (ARCHITECTURE.md: "Input source switching ... estimator reset").
    SetInput(Receiver<InputEvent>),
    /// No input source selected (or the previous one failed to open).
    ClearInput,
    SetGlobalOffsetMs(f64),
    /// Registers (or replaces) a connected output, ready to be scheduled.
    AddOutput {
        name: String,
        output: Box<dyn ClockOutput>,
        trim_ms: f64,
        forward_transport: bool,
    },
    /// Disables and disconnects an output (drops it, closing the port).
    RemoveOutput(String),
    SetOutputTrimMs {
        name: String,
        trim_ms: f64,
    },
    SetOutputForwardTransport {
        name: String,
        forward_transport: bool,
    },
    SetResyncQuantumBars(u32),
    SetForwardContinue(bool),
    /// Manual "Resync" button: behaves like an incoming `0xFA`
    /// (ARCHITECTURE.md §5.4).
    ManualResync,
    /// Measure panel (PROMPTS.md Prompt 7): start reading real-world lead
    /// samples from a second, independently-selected MIDI input. Resets the
    /// running mean/p99.
    SetMeasureInput(Receiver<InputEvent>),
    /// No measure input selected (or it failed to open).
    ClearMeasureInput,
    Shutdown,
}

/// How often the engine drains input/commands and checks for a no-signal
/// timeout when nothing more urgent (an imminent output send) wakes it sooner
/// (ARCHITECTURE.md §3: "sleeps ... at most ~5 ms, to process input and commands").
const POLL_INTERVAL: Duration = Duration::from_millis(5);
/// Below this margin to a deadline, stop waiting on the command channel (whose
/// own timeout precision isn't sub-millisecond) and spin instead
/// (ARCHITECTURE.md §8, timing-and-jitter skill: "sleep until ~1ms before the
/// deadline, then spin").
const SPIN_THRESHOLD: Duration = Duration::from_millis(2);
/// How often the published snapshot is refreshed (ARCHITECTURE.md §3: "at most
/// ~30x/s").
const PUBLISH_INTERVAL: Duration = Duration::from_millis(33);
/// Upper bound on input events drained per loop iteration, so a very fast input
/// burst can't starve command processing or snapshot publishing.
const MAX_EVENTS_PER_ITERATION: usize = 256;
/// Upper bound on catch-up ticks sent per output per loop iteration: normal
/// operation needs at most one, this is only a safety net against a runaway
/// loop if something is badly wrong.
const MAX_CATCHUP_TICKS_PER_ITERATION: usize = 16;
/// Window size for the running send-jitter / measured-lead statistics.
const STATS_WINDOW: usize = 200;

/// Owns the engine thread: drains the active input source, feeds the estimator,
/// schedules and sends output ticks, and publishes a [`Snapshot`] the UI can
/// read without blocking the engine (ARCHITECTURE.md §3 Threads).
pub struct EngineHandle {
    command_tx: Sender<Command>,
    snapshot: Arc<ArcSwap<Snapshot>>,
    join: Option<JoinHandle<()>>,
}

impl EngineHandle {
    pub fn start() -> Self {
        let (command_tx, command_rx) = crossbeam_channel::unbounded();
        let snapshot = Arc::new(ArcSwap::from_pointee(Snapshot::default()));
        let thread_snapshot = Arc::clone(&snapshot);

        let join = thread::Builder::new()
            .name("engine".into())
            .spawn(move || run(command_rx, thread_snapshot, crate::clock::RealTime))
            .expect("failed to spawn engine thread");

        Self {
            command_tx,
            snapshot,
            join: Some(join),
        }
    }

    /// Switches the active input source, resetting the estimator. `rx` should be
    /// a fresh (or freshly-selected) channel of [`InputEvent`]s.
    pub fn set_input(&self, rx: Receiver<InputEvent>) {
        let _ = self.command_tx.send(Command::SetInput(rx));
    }

    /// Clears the active input source (no selection, or it failed to open).
    pub fn clear_input(&self) {
        let _ = self.command_tx.send(Command::ClearInput);
    }

    /// Sets the global offset (ARCHITECTURE.md §5.3), in milliseconds, positive
    /// = earlier. Applied gradually (slewed) to each output.
    pub fn set_global_offset_ms(&self, offset_ms: f64) {
        let _ = self.command_tx.send(Command::SetGlobalOffsetMs(offset_ms));
    }

    /// Registers a connected, enabled output under `name` (replacing any
    /// previous output of the same name). The caller constructs the concrete
    /// [`ClockOutput`] (via `output.rs`); the engine only ever sees the trait
    /// object, never the concrete `vmidi`/`midir` types.
    pub fn add_output(
        &self,
        name: String,
        output: Box<dyn ClockOutput>,
        trim_ms: f64,
        forward_transport: bool,
    ) {
        let _ = self.command_tx.send(Command::AddOutput {
            name,
            output,
            trim_ms,
            forward_transport,
        });
    }

    /// Disables and disconnects (drops) the named output.
    pub fn remove_output(&self, name: String) {
        let _ = self.command_tx.send(Command::RemoveOutput(name));
    }

    /// Updates an already-registered output's trim; a no-op if it isn't
    /// currently registered (e.g. it's disabled).
    pub fn set_output_trim_ms(&self, name: String, trim_ms: f64) {
        let _ = self
            .command_tx
            .send(Command::SetOutputTrimMs { name, trim_ms });
    }

    /// Updates an already-registered output's forward-start/stop setting; a
    /// no-op if it isn't currently registered.
    pub fn set_output_forward_transport(&self, name: String, forward_transport: bool) {
        let _ = self.command_tx.send(Command::SetOutputForwardTransport {
            name,
            forward_transport,
        });
    }

    /// Sets the resync quantum, in bars (ARCHITECTURE.md §5.4: 1, 2 or 4).
    pub fn set_resync_quantum_bars(&self, quantum_bars: u32) {
        let _ = self
            .command_tx
            .send(Command::SetResyncQuantumBars(quantum_bars));
    }

    /// Enables/disables forwarding incoming `0xFB` Continue (ARCHITECTURE.md §5.1).
    pub fn set_forward_continue(&self, forward: bool) {
        let _ = self.command_tx.send(Command::SetForwardContinue(forward));
    }

    /// Manual "Resync" button: behaves like an incoming `0xFA` (ARCHITECTURE.md §5.4).
    pub fn manual_resync(&self) {
        let _ = self.command_tx.send(Command::ManualResync);
    }

    /// Measure panel (PROMPTS.md Prompt 7): starts reading real-world lead
    /// samples from `rx`, a fresh channel from a second, independently opened
    /// MIDI input (not the main clock pipeline).
    pub fn set_measure_input(&self, rx: Receiver<InputEvent>) {
        let _ = self.command_tx.send(Command::SetMeasureInput(rx));
    }

    /// Clears the measure input (no selection, or it failed to open).
    pub fn clear_measure_input(&self) {
        let _ = self.command_tx.send(Command::ClearMeasureInput);
    }

    /// The latest published snapshot (BPM, lock state, jitter, output status).
    pub fn snapshot(&self) -> Arc<Snapshot> {
        self.snapshot.load_full()
    }
}

impl Drop for EngineHandle {
    fn drop(&mut self) {
        let _ = self.command_tx.send(Command::Shutdown);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

struct OutputEntry {
    output: Box<dyn ClockOutput>,
    scheduler: scheduler::OutputScheduler,
    trim_secs: f64,
    /// ARCHITECTURE.md §6/§5.4: "forward start/stop" (default on). If off, the
    /// output only ever receives `0xF8` — never `0xFA`/`0xFC` — and its ticks
    /// are not paused by a `0xFC` either.
    forward_transport: bool,
}

/// Fixed-size ring buffer computing a running mean and 99th percentile in
/// O(n log n) per query (only called at snapshot-publish rate, never on a hot
/// path). Shared by send-jitter (ARCHITECTURE.md §8) and the Measure panel's
/// lead stats (PROMPTS.md Prompt 7) — both are "recent samples in, mean/p99
/// out", the only difference being that a measured lead may be negative
/// (arrived late) where a send-jitter magnitude never is.
#[derive(Debug, Clone)]
struct RingStats {
    buf: [f32; STATS_WINDOW],
    len: usize,
    pos: usize,
}

impl RingStats {
    fn new() -> Self {
        Self {
            buf: [0.0; STATS_WINDOW],
            len: 0,
            pos: 0,
        }
    }

    /// Records one sample, in ms. O(1), allocation-free.
    fn push(&mut self, value_ms: f64) {
        self.buf[self.pos] = value_ms as f32;
        self.pos = (self.pos + 1) % STATS_WINDOW;
        self.len = (self.len + 1).min(STATS_WINDOW);
    }

    fn mean_ms(&self) -> f64 {
        if self.len == 0 {
            return 0.0;
        }
        self.buf[..self.len].iter().map(|&v| v as f64).sum::<f64>() / self.len as f64
    }

    fn p99_ms(&self) -> f64 {
        if self.len == 0 {
            return 0.0;
        }
        let mut sorted = self.buf;
        let samples = &mut sorted[..self.len];
        // `total_cmp`, not `partial_cmp().unwrap()`: send-jitter magnitudes
        // and measured leads are always finite, but a total order costs
        // nothing here and never panics.
        samples.sort_by(|a, b| a.total_cmp(b));
        let idx = (((self.len as f64) * 0.99).ceil() as usize)
            .saturating_sub(1)
            .min(self.len - 1);
        samples[idx] as f64
    }
}

/// Default resync quantum before any `SetResyncQuantumBars` command arrives
/// (ARCHITECTURE.md §5.4: "default 1").
const DEFAULT_RESYNC_QUANTUM_BARS: u32 = 1;

/// All engine state a [`Command`] can touch, bundled so `apply_command` and
/// the transport helpers below don't need an ever-growing parameter list.
struct EngineState {
    input_rx: Option<Receiver<InputEvent>>,
    estimator: estimator::Estimator,
    global_offset_secs: f64,
    outputs: HashMap<String, OutputEntry>,
    resync_quantum_bars: u32,
    forward_continue: bool,
    run_state: transport::RunState,
    /// A start arrived while the estimator wasn't `Locked` yet; armed as soon
    /// as it locks on (ARCHITECTURE.md §5.4: "wait until Locked").
    awaiting_start_lock: bool,
    /// The position counter's origin (input tick index `k_d` of the last
    /// processed downbeat) — ARCHITECTURE.md §5.4: "tick 0 of bar 0".
    bar0_tick: Option<u64>,
    /// Measure panel (PROMPTS.md Prompt 7): a second, independent MIDI input
    /// whose `Clock` arrival times are compared against the main input's
    /// predicted tick times, for real-world lead validation.
    measure_rx: Option<Receiver<InputEvent>>,
    measure_stats: RingStats,
}

impl EngineState {
    fn new() -> Self {
        Self {
            input_rx: None,
            estimator: estimator::Estimator::new(),
            global_offset_secs: 0.0,
            outputs: HashMap::new(),
            resync_quantum_bars: DEFAULT_RESYNC_QUANTUM_BARS,
            forward_continue: false,
            run_state: transport::RunState::Running,
            awaiting_start_lock: false,
            bar0_tick: None,
            measure_rx: None,
            measure_stats: RingStats::new(),
        }
    }
}

/// Applies one command to engine state. Returns `true` if the engine should
/// shut down.
fn apply_command(command: Command, state: &mut EngineState, now: Instant) -> bool {
    match command {
        Command::SetInput(rx) => {
            state.input_rx = Some(rx);
            state.estimator.reset();
        }
        Command::ClearInput => {
            state.input_rx = None;
            state.estimator.reset();
        }
        Command::SetGlobalOffsetMs(offset_ms) => {
            state.global_offset_secs = offset_ms / 1000.0;
        }
        Command::AddOutput {
            name,
            output,
            trim_ms,
            forward_transport,
        } => {
            state.outputs.insert(
                name,
                OutputEntry {
                    output,
                    scheduler: scheduler::OutputScheduler::new(),
                    trim_secs: trim_ms / 1000.0,
                    forward_transport,
                },
            );
        }
        Command::RemoveOutput(name) => {
            state.outputs.remove(&name);
        }
        Command::SetOutputTrimMs { name, trim_ms } => {
            if let Some(entry) = state.outputs.get_mut(&name) {
                entry.trim_secs = trim_ms / 1000.0;
            }
        }
        Command::SetOutputForwardTransport {
            name,
            forward_transport,
        } => {
            if let Some(entry) = state.outputs.get_mut(&name) {
                entry.forward_transport = forward_transport;
            }
        }
        Command::SetResyncQuantumBars(quantum_bars) => {
            state.resync_quantum_bars = quantum_bars;
        }
        Command::SetForwardContinue(forward) => {
            state.forward_continue = forward;
        }
        Command::ManualResync => begin_start(state, now),
        Command::SetMeasureInput(rx) => {
            state.measure_rx = Some(rx);
            state.measure_stats = RingStats::new();
        }
        Command::ClearMeasureInput => {
            state.measure_rx = None;
            state.measure_stats = RingStats::new();
        }
        Command::Shutdown => return true,
    }
    false
}

/// Handles an incoming (or manual) start (ARCHITECTURE.md §5.4): if the
/// estimator is already `Locked`, arms a pending `0xFA` on every
/// forward-transport-enabled output immediately; otherwise defers until it
/// locks on (checked each iteration in [`run`]).
fn begin_start(state: &mut EngineState, now: Instant) {
    state.run_state = transport::RunState::Running;
    if state.estimator.lock_state() == estimator::LockState::Locked
        && let Some(model) = state.estimator.model()
    {
        arm_all_outputs(&model, now, state);
        state.awaiting_start_lock = false;
    } else {
        state.awaiting_start_lock = true;
    }
}

/// Arms a pending start on every forward-transport-enabled output, and sets
/// the shared position counter origin (ARCHITECTURE.md §5.4).
fn arm_all_outputs(model: &estimator::TickModel, now: Instant, state: &mut EngineState) {
    let k_d = model.next_k;
    let quantum_ticks = transport::quantum_ticks(state.resync_quantum_bars);
    state.bar0_tick = Some(k_d);
    for entry in state.outputs.values_mut() {
        if !entry.forward_transport {
            continue;
        }
        let shift = state.global_offset_secs + entry.trim_secs;
        let earliest_unsent = entry.scheduler.next_unsent_k(model);
        let target =
            transport::resync_target(model, shift, k_d, quantum_ticks, earliest_unsent, now);
        entry.scheduler.arm_start(target);
    }
}

/// Handles an incoming `0xFC` (ARCHITECTURE.md §5.4): forwards immediately to
/// forward-transport-enabled outputs, cancels any pending starts, and stops
/// their ticks until the next start.
fn handle_stop(state: &mut EngineState) {
    state.run_state = transport::RunState::Stopped;
    state.awaiting_start_lock = false;
    for entry in state.outputs.values_mut() {
        if !entry.forward_transport {
            continue;
        }
        entry.scheduler.cancel_start();
        if let Err(err) = entry.output.send(&[STOP]) {
            log::warn!("output '{}' send failed: {err}", entry.output.name());
        }
    }
}

fn run(command_rx: Receiver<Command>, snapshot: Arc<ArcSwap<Snapshot>>, clock: impl TimeSource) {
    crate::timing::set_current_thread_time_critical();

    let mut state = EngineState::new();
    let mut send_jitter = RingStats::new();
    let mut last_publish = clock.now();

    loop {
        loop {
            match command_rx.try_recv() {
                Ok(command) => {
                    if apply_command(command, &mut state, clock.now()) {
                        return;
                    }
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return,
            }
        }

        if let Some(rx) = &state.input_rx {
            // Borrowed separately below (`state.estimator`/`state.outputs`),
            // so take it out for the duration of the drain loop.
            let rx = rx.clone();
            for _ in 0..MAX_EVENTS_PER_ITERATION {
                match rx.try_recv() {
                    Ok(event) => match event.msg {
                        RtMsg::Clock => state.estimator.on_tick(event.at),
                        RtMsg::Start => begin_start(&mut state, clock.now()),
                        RtMsg::Continue => {
                            if state.forward_continue {
                                for entry in state.outputs.values_mut() {
                                    if entry.forward_transport
                                        && let Err(err) = entry.output.send(&[CONTINUE])
                                    {
                                        log::warn!(
                                            "output '{}' send failed: {err}",
                                            entry.output.name()
                                        );
                                    }
                                }
                            }
                        }
                        RtMsg::Stop => handle_stop(&mut state),
                    },
                    Err(_) => break,
                }
            }
        }

        state.estimator.check_timeout(clock.now());
        let model = state.estimator.model();

        // Measure panel (PROMPTS.md Prompt 7): correlate each Clock arriving
        // on the independent measure input with the closest predicted main-
        // input tick, recording the (possibly negative) real-world lead.
        // Drained even without a model yet, so a bounded channel can't back
        // up while waiting for the main input to lock.
        if let Some(rx) = &state.measure_rx {
            let rx = rx.clone();
            for _ in 0..MAX_EVENTS_PER_ITERATION {
                match rx.try_recv() {
                    Ok(event) => {
                        if event.msg == RtMsg::Clock
                            && let Some(model) = &model
                        {
                            let k = model.nearest_tick_index(event.at);
                            let lead_ms =
                                estimator::signed_millis(model.predicted_instant(k), event.at);
                            state.measure_stats.push(lead_ms);
                        }
                    }
                    Err(_) => break,
                }
            }
        }

        // A start arrived before the estimator was `Locked`: arm it now if
        // it just became so (ARCHITECTURE.md §5.4: "wait until Locked").
        if state.awaiting_start_lock
            && state.estimator.lock_state() == estimator::LockState::Locked
            && let Some(model) = &model
        {
            arm_all_outputs(model, clock.now(), &mut state);
            state.awaiting_start_lock = false;
        }

        // Send any due ticks (ARCHITECTURE.md §5.3: "all due messages for the
        // same instant are sent in one pass"), and track the earliest
        // still-pending send time so we know how long we can sleep.
        let mut earliest_due: Option<Instant> = None;
        match &model {
            Some(model) => {
                for entry in state.outputs.values_mut() {
                    if entry.forward_transport && state.run_state == transport::RunState::Stopped {
                        // ARCHITECTURE.md §5.4: stopped until the next start.
                        entry.scheduler.reset();
                        continue;
                    }

                    let shift = state.global_offset_secs + entry.trim_secs;
                    for _ in 0..MAX_CATCHUP_TICKS_PER_ITERATION {
                        let (k, send_at) = entry.scheduler.peek_next(model, shift);
                        let now = clock.now();
                        if send_at > now {
                            earliest_due = Some(match earliest_due {
                                Some(existing) if existing <= send_at => existing,
                                _ => send_at,
                            });
                            break;
                        }

                        if entry.forward_transport
                            && entry.scheduler.take_due_start(k)
                            && let Err(err) = entry.output.send(&[START])
                        {
                            log::warn!("output '{}' send failed: {err}", entry.output.name());
                        }

                        if let Err(err) = entry.output.send(&[CLOCK]) {
                            log::warn!("output '{}' send failed: {err}", entry.output.name());
                        }
                        let actual = clock.now();
                        send_jitter
                            .push(actual.saturating_duration_since(send_at).as_secs_f64() * 1000.0);
                        entry.scheduler.commit(k, actual, shift);
                    }
                }
            }
            None => {
                // NoSignal, or still collecting the first samples: stop
                // sending (ARCHITECTURE.md §5.3) and forget tick positions so
                // outputs restart cleanly once a model reappears.
                for entry in state.outputs.values_mut() {
                    entry.scheduler.reset();
                }
            }
        }

        let now = clock.now();
        if now.saturating_duration_since(last_publish) >= PUBLISH_INTERVAL {
            let pending_start = state.awaiting_start_lock
                || state
                    .outputs
                    .values()
                    .any(|entry| entry.forward_transport && entry.scheduler.has_pending_start());
            let output_position = state.bar0_tick.and_then(|bar0| {
                let reference_k = state
                    .outputs
                    .values()
                    .filter(|entry| entry.forward_transport)
                    .filter_map(|entry| entry.scheduler.last_committed_k())
                    .max()?;
                let elapsed = reference_k.checked_sub(bar0)?;
                let quantum_ticks = transport::quantum_ticks(state.resync_quantum_bars);
                Some(OutputPosition {
                    tick_in_bar: (elapsed % transport::TICKS_PER_BAR) as u32,
                    bar_in_phrase: ((elapsed % quantum_ticks) / transport::TICKS_PER_BAR) as u32,
                })
            });

            snapshot.store(Arc::new(Snapshot {
                lock_state: state.estimator.lock_state(),
                bpm: state.estimator.bpm(),
                input_jitter_ms: state.estimator.jitter_ms(),
                send_jitter_p99_ms: send_jitter.p99_ms(),
                outputs: state
                    .outputs
                    .iter()
                    .map(|(name, entry)| OutputStatus {
                        name: name.clone(),
                        connected: entry.output.is_connected(),
                    })
                    .collect(),
                pending_start,
                output_position,
                measure_lead_mean_ms: state.measure_stats.mean_ms(),
                measure_lead_p99_ms: state.measure_stats.p99_ms(),
            }));
            last_publish = now;
        }

        // Wait until the earliest due output send, capped by POLL_INTERVAL so
        // input/commands stay responsive even with nothing scheduled.
        let now = clock.now();
        let wake_at = match earliest_due {
            Some(due) => due.min(now + POLL_INTERVAL),
            None => now + POLL_INTERVAL,
        };
        let budget = wake_at.saturating_duration_since(now);

        if budget > SPIN_THRESHOLD {
            // Coarse wait: also wakes early (and applies) if a command
            // arrives. Any command received here must still be applied - one
            // that arrives during this wait is otherwise silently dequeued and
            // lost, never seen by the `try_recv` loop above on the next
            // iteration.
            match command_rx.recv_timeout(budget - SPIN_THRESHOLD) {
                Ok(command) => {
                    if apply_command(command, &mut state, clock.now()) {
                        return;
                    }
                    continue;
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
            let now = clock.now();
            if wake_at > now {
                spin_sleep::sleep(wake_at - now);
            }
        } else if budget > Duration::ZERO {
            spin_sleep::sleep(budget);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::Duration;

    use super::*;
    use crate::output::OutputError;

    /// Regression test: a command sent while the engine thread is blocked in
    /// its `recv_timeout` wait must still be applied, not silently dequeued and
    /// discarded (the bug this guards against: `let _ = command_rx.recv_timeout(..)`
    /// with the result thrown away, which drops whatever command that call
    /// happened to receive). Uses real threads/timing deliberately: this is
    /// exercising cross-thread channel plumbing, not the estimator's timing
    /// precision, so it isn't the kind of test the "no real wall-clock timing"
    /// rule is about.
    #[test]
    fn set_input_and_a_tick_are_not_lost_even_if_sent_during_the_poll_wait() {
        let handle = EngineHandle::start();
        // Give the engine thread a chance to reach its `recv_timeout` wait
        // before we send anything, to specifically exercise that code path.
        std::thread::sleep(Duration::from_millis(2));

        let (tx, rx) = crossbeam_channel::bounded(16);
        handle.set_input(rx);
        tx.send(InputEvent {
            at: Instant::now(),
            msg: RtMsg::Clock,
        })
        .unwrap();

        let mut observed = estimator::LockState::NoSignal;
        for _ in 0..100 {
            observed = handle.snapshot().lock_state;
            if observed != estimator::LockState::NoSignal {
                break;
            }
            std::thread::sleep(POLL_INTERVAL);
        }

        assert_eq!(
            observed,
            estimator::LockState::Acquiring,
            "the tick should have reached the estimator within ~{:?}",
            POLL_INTERVAL * 100
        );
    }

    struct RecordingOutput {
        name: String,
        sent_count: Arc<AtomicU32>,
        /// Every byte ever sent, in order — lets tests check *what* was sent
        /// (e.g. that a `0xFA`/`0xFC` never reaches a `forward_transport =
        /// false` output), not just how often.
        sent_bytes: Arc<std::sync::Mutex<Vec<u8>>>,
    }

    impl RecordingOutput {
        fn new(name: &str) -> (Self, Arc<AtomicU32>, Arc<std::sync::Mutex<Vec<u8>>>) {
            let sent_count = Arc::new(AtomicU32::new(0));
            let sent_bytes = Arc::new(std::sync::Mutex::new(Vec::new()));
            (
                Self {
                    name: name.to_string(),
                    sent_count: Arc::clone(&sent_count),
                    sent_bytes: Arc::clone(&sent_bytes),
                },
                sent_count,
                sent_bytes,
            )
        }
    }

    impl ClockOutput for RecordingOutput {
        fn name(&self) -> &str {
            &self.name
        }

        fn send(&mut self, msg: &[u8]) -> Result<(), OutputError> {
            self.sent_count.fetch_add(1, Ordering::Relaxed);
            self.sent_bytes.lock().unwrap().extend_from_slice(msg);
            Ok(())
        }

        fn is_connected(&self) -> bool {
            true
        }
    }

    /// End-to-end functional check (real threads, real `EngineHandle`, no
    /// timing-precision assertions per the "no real wall-clock timing in
    /// `cargo test`" rule): a registered output actually receives ticks once
    /// the estimator locks onto a steady input stream. This is the kind of
    /// wiring bug (channel silently never drained, output never scheduled)
    /// that pure `scheduler`/`estimator` unit tests can't catch on their own.
    #[test]
    fn added_output_actually_receives_ticks_end_to_end() {
        let handle = EngineHandle::start();
        let (tx, rx) = crossbeam_channel::bounded(1024);
        handle.set_input(rx);
        handle.set_global_offset_ms(50.0);

        let (output, sent_count, _sent_bytes) = RecordingOutput::new("test-output");
        handle.add_output("test-output".to_string(), Box::new(output), 0.0, true);

        // Fast tempo, so locking (needs 24 ticks) takes little real time.
        let period = Duration::from_secs_f64(60.0 / (300.0 * 24.0));
        let mut next = Instant::now();
        for _ in 0..80 {
            next += period;
            let now = Instant::now();
            if next > now {
                std::thread::sleep(next - now);
            }
            let _ = tx.send(InputEvent {
                at: Instant::now(),
                msg: RtMsg::Clock,
            });
        }
        std::thread::sleep(Duration::from_millis(50));

        assert!(
            sent_count.load(Ordering::Relaxed) > 0,
            "the registered output should have received at least one Clock tick"
        );
    }

    /// End-to-end wiring check for the manual "Resync" button and per-output
    /// `forward_transport` (ARCHITECTURE.md §5.4): once locked, a manual
    /// resync must make a `forward_transport = true` output actually receive
    /// an `0xFA`, while a `forward_transport = false` output on the same
    /// engine never sees one (real threads/timing, no precision assertions —
    /// same rationale as `added_output_actually_receives_ticks_end_to_end`).
    #[test]
    fn manual_resync_sends_start_only_to_forwarding_outputs() {
        let handle = EngineHandle::start();
        let (tx, rx) = crossbeam_channel::bounded(1024);
        handle.set_input(rx);
        handle.set_global_offset_ms(0.0);
        handle.set_resync_quantum_bars(1);

        let (fwd_output, _fwd_count, fwd_bytes) = RecordingOutput::new("forwarding");
        handle.add_output("forwarding".to_string(), Box::new(fwd_output), 0.0, true);
        let (silent_output, _silent_count, silent_bytes) = RecordingOutput::new("silent");
        handle.add_output("silent".to_string(), Box::new(silent_output), 0.0, false);

        // Fast tempo, so locking (needs 24 ticks) and clearing the jitter
        // threshold takes little real time.
        let period = Duration::from_secs_f64(60.0 / (300.0 * 24.0));
        let mut next = Instant::now();
        for _ in 0..400 {
            next += period;
            let now = Instant::now();
            if next > now {
                std::thread::sleep(next - now);
            }
            let _ = tx.send(InputEvent {
                at: Instant::now(),
                msg: RtMsg::Clock,
            });
            if handle.snapshot().lock_state == estimator::LockState::Locked {
                break;
            }
        }
        assert_eq!(handle.snapshot().lock_state, estimator::LockState::Locked);

        handle.manual_resync();

        for _ in 0..200 {
            next += period;
            let now = Instant::now();
            if next > now {
                std::thread::sleep(next - now);
            }
            let _ = tx.send(InputEvent {
                at: Instant::now(),
                msg: RtMsg::Clock,
            });
        }
        std::thread::sleep(Duration::from_millis(50));

        assert!(
            fwd_bytes.lock().unwrap().contains(&START),
            "the forwarding output should have received a 0xFA after resync"
        );
        assert!(
            !silent_bytes.lock().unwrap().contains(&START),
            "an output with forward_transport = false must never receive 0xFA"
        );
        assert!(
            silent_bytes.lock().unwrap().contains(&CLOCK),
            "an output with forward_transport = false should still receive 0xF8"
        );
    }

    /// PROMPTS.md Prompt 7 stress test: drives `Estimator` + `OutputScheduler`
    /// directly (no `EngineHandle`, no real threads, no real waiting) through
    /// 10 *simulated* minutes of varying tempo and jitter, event-driven off
    /// `crate::sim::Simulator` (reused rather than a real-time thread — see
    /// `SimulatorHandle::spawn`/`run` for the real-time wrapper this skips).
    /// Runs in a small fraction of a real second despite covering 10 minutes
    /// of clock time, in line with "never assert real wall-clock timing" and
    /// "long-running stress tests use simulated time" (timing-and-jitter
    /// skill). `#[ignore]`d per PROMPTS.md ("cargo test --release -- --ignored").
    #[test]
    #[ignore = "10 simulated minutes of ticks; run explicitly via `cargo test --release -- --ignored`"]
    fn stress_ten_simulated_minutes_of_varying_tempo_and_jitter() {
        use crate::sim::{SharedParams, Simulator};

        const OFFSET_SECS: f64 = 0.2;
        const SIM_DURATION: Duration = Duration::from_secs(600);
        // Gentle BPM/jitter hops (not tempo jumps large enough to force
        // outlier-triggered re-acquisition) so ticks flow continuously
        // through every phase, per the PLL's smooth-tracking design
        // (timing-and-jitter skill point 2).
        const PROFILE: [(f64, f64); 5] = [
            (120.0, 0.0),
            (140.0, 2.0),
            (100.0, 1.0),
            (160.0, 3.0),
            (120.0, 0.5),
        ];
        const PHASE_DURATION: Duration = Duration::from_secs(120); // 5 * 120s = 600s

        let epoch = Instant::now();
        let params = Arc::new(SharedParams::new(PROFILE[0].0, PROFILE[0].1));
        let mut sim = Simulator::new(Arc::clone(&params), epoch, 0xC0FFEE_u64);

        let mut estimator = estimator::Estimator::new();
        let mut sched = scheduler::OutputScheduler::new();
        let mut last_committed: Option<(u64, Instant)> = None;
        let mut lead_samples_ms: Vec<f64> = Vec::new();
        let mut next_phase_switch = epoch + PHASE_DURATION;
        let mut phase = 0usize;

        loop {
            let event = sim.next_tick();
            if event.at.duration_since(epoch) >= SIM_DURATION {
                break;
            }

            if event.at >= next_phase_switch {
                phase = (phase + 1) % PROFILE.len();
                let (bpm, jitter_ms) = PROFILE[phase];
                params.set(bpm, jitter_ms);
                next_phase_switch += PHASE_DURATION;
            }

            // Drain any output ticks due strictly before this new input tick,
            // using the model as it stood *before* this tick updates it —
            // between two real input ticks the model doesn't change, so this
            // mirrors the real engine processing everything in arrival order.
            if let Some(model) = estimator.model() {
                loop {
                    let (k, send_at) = sched.peek_next(&model, OFFSET_SECS);
                    if send_at > event.at {
                        break;
                    }
                    if let Some((last_k, last_at)) = last_committed {
                        assert_eq!(
                            k,
                            last_k + 1,
                            "tick index must be strictly monotonic, no gaps or duplicates"
                        );
                        let min_gap = model.period().mul_f64(0.5);
                        assert!(
                            send_at.duration_since(last_at) >= min_gap,
                            "ticks sent closer than half a period: {last_at:?} -> {send_at:?} \
                             (min gap {min_gap:?})"
                        );
                    }
                    sched.commit(k, send_at, OFFSET_SECS);
                    let predicted = model.predicted_instant(k);
                    lead_samples_ms.push(estimator::signed_millis(predicted, send_at));
                    last_committed = Some((k, send_at));
                }
            } else {
                // No model (still (re-)collecting the initial fit): mirror
                // the real engine (`run`'s `None` branch) — forget the tick
                // position so numbering restarts cleanly from the new model
                // once it's ready, instead of resuming from a now-meaningless
                // stale `k` under an entirely new epoch/tick-index origin.
                sched.reset();
                last_committed = None;
            }

            estimator.on_tick(event.at);
        }

        assert!(
            lead_samples_ms.len() > 10_000,
            "expected many thousands of output ticks over 10 simulated minutes, got {}",
            lead_samples_ms.len()
        );
        let mean_lead_ms = lead_samples_ms.iter().sum::<f64>() / lead_samples_ms.len() as f64;
        let target_ms = OFFSET_SECS * 1000.0;
        assert!(
            (mean_lead_ms - target_ms).abs() < 1.0,
            "mean lead {mean_lead_ms:.4}ms should be within 1ms of the {target_ms}ms offset"
        );
    }
}
