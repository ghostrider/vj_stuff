mod app;
mod clock;
mod config;
mod engine;
mod logging;
mod output;
mod ports;
mod sim;
mod timing;
mod vmidi;

use std::path::PathBuf;

use clap::Parser;
use eframe::egui;

const DEFAULT_WINDOW_SIZE: [f32; 2] = [1440.0, 770.0];

/// midi_clock_shifter: receives MIDI clock, predicts it, and re-emits it with a phase offset.
#[derive(Parser, Debug)]
#[command(name = "midi_clock_shifter", version, about)]
struct Cli {
    /// Run the built-in simulator at the given BPM instead of a real input source.
    #[arg(long, value_name = "BPM")]
    simulate: Option<f64>,

    /// Config file path (defaults to %APPDATA%\midi_clock_shifter\config.json).
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,
}

fn main() -> eframe::Result {
    // First thing, so nothing else's log lines are dropped (ARCHITECTURE.md §9:
    // "Log panel (last 200 lines, timestamped)").
    let log_buffer = logging::init();
    log::info!("midi_clock_shifter v{} starting", env!("CARGO_PKG_VERSION"));

    let cli = Cli::parse();

    let mut config = config::AppConfig::load(cli.config.as_deref());
    if let Some(bpm) = cli.simulate {
        config.input_source = config::InputSource::Simulator;
        config.simulator.bpm = bpm;
    }
    let config_path = cli.config.clone();

    // Kept alive for the whole app lifetime; raises the Windows timer resolution to
    // 1ms at startup and restores it on drop (ARCHITECTURE.md §8).
    let _timer_guard = timing::TimerResolutionGuard::new();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size(DEFAULT_WINDOW_SIZE),
        ..Default::default()
    };

    eframe::run_native(
        "midi_clock_shifter",
        native_options,
        Box::new(move |cc| {
            // Always dark (ARCHITECTURE.md §9) rather than following the OS theme.
            cc.egui_ctx.set_theme(egui::ThemePreference::Dark);
            Ok(Box::new(app::App::new(config, config_path, log_buffer)))
        }),
    )
}
