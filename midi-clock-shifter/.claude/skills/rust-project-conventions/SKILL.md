---
name: rust-project-conventions
description: Structure, dependency, and style conventions for the Midi Clock Shifter Rust app — use whenever adding modules, choosing a crate/dependency, structuring errors, or deciding where new code belongs in this project.
---

# Midi Clock Shifter — Rust Conventions

The full design is in `ARCHITECTURE.md` (authoritative). This skill summarises it and adds coding conventions.

## What this app is

A Windows desktop app (Rust, egui, crate `midi_clock_shifter`) that receives MIDI clock, predicts it, and re-emits it with a **phase offset** — normally ahead of time — to one or more MIDI outputs. It compensates the video latency (~200 ms) between Resolume and the projector. The clock source is a BPM detection app; the destinations are Resolume and hardware sequencers (Beatstep Pro, KeyStep, …) that trigger Resolume clips. See [[midi-clock-protocol]] for what "shift" means here. Tempo scaling is out of scope.

### UI requirements

- The app detects MIDI devices automatically (polling every 2 s, auto-reconnect by port name).
- Layout: **one toolbar and two panels**.
- **Toolbar:** offset in milliseconds, entered as a positive or negative number or via a slider, range **−4000 … +4000 ms** (one 4/4 bar at 60 BPM; positive = earlier, default +200 ms; fine slider resolution around 0–500 ms, offset also shown in beats at the current tempo); resync quantum; Resync button; lock state, BPM and jitter display.
- **Left panel – input:** the user selects exactly **one** input source (single selection, radio buttons). Sources: the app's own virtual port `Clock Shifter In` (default, first row, the BPM detection app sends to it), any hardware MIDI input, or the simulator.
- **Right panel – outputs:** the user selects **one or several** outputs (checkboxes) that receive the shifted clock. `Clock Shifter Out` (the app's own virtual port, read by Resolume) is always the first row. Each output has a trim (±50 ms) and a "forward start/stop" option.
- The app's own virtual ports never appear in the wrong panel (loop protection).

## Module layout

Keep the timing-critical logic isolated from I/O and UI so it can be tested without real ports:

```
src/
├── main.rs            # clap args, thread wiring, eframe launch
├── app.rs             # egui UI (reads snapshots, sends commands)
├── config.rs          # AppConfig, load/save JSON
├── vmidi/             # virtualMIDI SDK: ffi.rs (libloading) + safe VirtualPort
├── ports.rs           # midir enumeration, hot-plug polling, filtering
├── engine/
│   ├── estimator.rs   # tempo/phase model (PLL), lock state
│   ├── transport.rs   # start/stop/resync, bar/phrase counting
│   └── scheduler.rs   # per-output send times, slewing
├── output.rs          # trait ClockOutput + VirtualOut / MidirOut
├── timing.rs          # timer resolution, thread priority, precise sleep
├── clock.rs           # TimeSource trait (real + simulated)
└── sim.rs             # clock simulator
```

`engine/` and `clock.rs` must not know any concrete MIDI crate. They take `InputEvent`s (`Instant` + `RtMsg` enum) and talk to outputs only through the `ClockOutput` trait. This keeps them unit-testable with the simulated `TimeSource` — see [[timing-and-jitter]].

## Dependency choices

- **Virtual ports:** virtualMIDI SDK (`teVirtualMIDI64.dll`) via `libloading`. The app creates its own ports; do not require the user to configure loopMIDI ports. Take FFI signatures and flag values from `teVirtualMIDI.h` only — never guess.
- **Hardware MIDI I/O:** `midir`.
- **UI:** `eframe`/`egui`.
- **Channels / shared state:** `crossbeam-channel` (bounded), `arc-swap` for the UI snapshot.
- **Timing:** `spin_sleep`, `windows` crate (timer resolution, thread priority).
- **Config:** `serde`, `serde_json`, `directories`.
- **CLI:** `clap` (derive) for the few flags (`--simulate <bpm>`, `--config <path>`).
- **Errors:** `anyhow` in `main.rs` and the UI; `thiserror` for typed errors in `engine/`, `vmidi/` and `output.rs`. No `unwrap()`/`expect()` outside `main.rs` startup, tests, or genuinely impossible states. MIDI/FFI errors go to the log, never panic.
- **No async runtime** (tokio, async-std).
- Keep the dependency list small; ask before adding anything not listed here.

## Style

- `cargo fmt` and `cargo clippy --all-targets -- -D warnings` must be clean before a change is done. Fix warnings instead of `#[allow]`-ing them, unless there is a specific, commented reason.
- Named constants for protocol values (`const CLOCK: u8 = 0xF8;`), no bare hex literals in logic.
- Settings for outputs and inputs are keyed by **port name**, never by index.
- The hot path (callbacks, and everything between computing a send time and sending) is allocation-, lock- and logging-free — see [[timing-and-jitter]] before adding anything there.
