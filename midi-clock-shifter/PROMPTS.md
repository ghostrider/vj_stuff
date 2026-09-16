# midi_clock_shifter – Prompts for Claude Code

Execute one prompt at a time. After each prompt: `cargo fmt`, `cargo build --release`
and `cargo clippy --all-targets -- -D warnings` must be clean, and the listed acceptance checks must
pass. Do not start the next prompt until the user confirms.

Always follow ARCHITECTURE.md. If something in it is technically wrong
(e.g. an FFI signature), stop, explain, and propose a correction before coding.

---

## Prompt 1 – Project skeleton, config, UI shell

Update this cargo project with the module layout from ARCHITECTURE.md §3
(empty modules with TODOs where logic comes later).

Implement:
- `config.rs`: `AppConfig` with global offset (default +200 ms, range −4000 … +4000 ms,
  positive = earlier), input source selection, resync quantum
  (default 1 bar), virtual port names (`Clock Shifter In`, `Clock Shifter Out`),
  per-output settings map keyed by port name (`enabled`, `trim_ms`,
  `forward_transport`), continue handling, simulator settings. Load/save JSON in
  `%APPDATA%\midi_clock_shifter\config.json`, fall back to defaults on any error,
  debounced save.
- `ports.rs`: enumerate `midir` input and output ports.
- `main.rs`: `clap` args (`--simulate <bpm>`, `--config <path>`), wiring only.
- `app.rs`: egui window with the layout from §9 (toolbar, left panel = input source
  single selection, right panel = outputs multi selection). Values are editable and
  persisted, but nothing is wired to MIDI yet. Both panels show real `midir` ports.
- `timing.rs`: `TimerResolutionGuard` (timeBeginPeriod/timeEndPeriod).

Acceptance: app starts, lists MIDI outputs, settings survive a restart.

---

## Prompt 2 – virtualMIDI FFI and virtual ports

Implement `vmidi/ffi.rs` and `vmidi/mod.rs` per §4.

- Load `teVirtualMIDI64.dll` with `libloading`. Declare the functions exactly as in
  `teVirtualMIDI.h` (ask the user to provide the header path or contents if it is not
  available; do not guess signatures or flag values).
- Safe `VirtualPort` wrapper: create with name, flags and a callback that timestamps
  first and `try_send`s a `Copy` `InputEvent { at, msg: RtMsg }` into a bounded
  crossbeam channel (no allocation, see §3 Threads).
  Implement `send()` and `Drop`. Handle the "port closed" callback signal.
- Create `Clock Shifter In` (visible to other apps as an output only) and
  `Clock Shifter Out` (visible as an input only).
- If the DLL/driver is missing: app still starts, shows an error banner, logs the
  reason.
- Show driver and DLL version in the header.
- Temporary debug view: counter of received 0xF8/0xFA/0xFB/0xFC messages and the
  measured interval between the last two clock ticks.
- Temporary "Test send" button that sends 0xFA and a few 0xF8 on `Clock Shifter Out`.

Show `Clock Shifter In` as first entry in the input panel; hide the virtual ports from
the wrong panel (loop protection, §7).

Acceptance: the BPM detection app lists `Clock Shifter In`, sending clock to it
increments the counters; Resolume (or a MIDI monitor) sees `Clock Shifter Out` and
receives the test messages.

---

## Prompt 3 – Estimator and simulator

Implement `clock.rs` (TimeSource trait: real and simulated), `engine/estimator.rs`
per §5.2, and `sim.rs` per §10.

- Unit tests with simulated time:
  - steady 125 BPM with 0 and 2 ms jitter → locks within 48 ticks, BPM error < 0.05
  - tempo step 125 → 130 BPM → re-locks, no NaN, no panic
  - dropout of 1 s → state goes to NoSignal, recovers when ticks resume
  - single outlier tick (+15 ms) → model barely moves
- Hardware input source via `midir` input callback (same `InputEvent` path,
  per-byte real-time filtering incl. interleaved bytes and ignoring 0xFE).
- Input source switching from the left panel at runtime (close old, open new,
  estimator reset).
- Engine thread skeleton: drains the input channel (selected source),
  feeds the estimator, publishes a snapshot (BPM, lock state, input jitter).
- UI shows these values. Remove the debug interval display from Prompt 2.

Acceptance: tests pass; with the simulator and with the real BPM app, BPM and lock
state are shown correctly.

---

## Prompt 4 – Scheduler and outputs

Implement `output.rs` (trait + `VirtualOut` + `MidirOut`) and `engine/scheduler.rs`
per §5.3 and §6. Move all output connections into the engine thread.

- Per-output send times from the model with global offset and per-output trim.
- Strictly monotonic tick emission, catch-up rule, hold-back, slewing (max 2 ms per tick).
- Stop emitting when NoSignal.
- Precise waiting: thread priority TIME_CRITICAL, `spin_sleep`.
- Measure scheduled-vs-actual send error; show p99 "send jitter" in the UI.
- Enable/disable outputs and change trims live via commands from the UI.
- Remove the temporary test-send button.

Unit tests (simulated time):
- with offset 200 ms, every output tick k is sent ~200 ms before input tick k arrives
  (error < 1 ms in simulation)
- offset change 200 → 300 ms: no two consecutive ticks closer than 0.5·P,
  offset reached gradually
- offset change 300 → 200 ms: no tick sent twice, no negative intervals
- negative offset (−150 ms): ticks sent ~150 ms after predicted input arrival
- two outputs with different trims get independent, correct send times

Acceptance: tests pass; Resolume follows the tempo from `Clock Shifter Out`;
Beatstep Pro follows the tempo when enabled.

---

## Prompt 5 – Transport: start, stop, resync

Implement `engine/transport.rs` per §5.4.

- Position counter (tick in bar, bar in phrase), resync quantum 1/2/4 bars.
- Incoming 0xFA → pending output start before output tick `k_d + Q`, ticks keep
  running, a newer start replaces a pending one.
- Start from stopped state: wait for Locked, then start at the next boundary.
- Incoming 0xFC → forward immediately (only to outputs with forward_transport),
  cancel pending start, stop ticks until next start.
- 0xFB ignored unless enabled in settings.
- Manual "Resync" button behaves like an incoming start.
- Per-output `forward_transport` option.
- UI: beat cells + bar/phrase counter of the **output** position, beat 1 flashes;
  pending-start indicator.

Unit tests:
- after an incoming start, each output receives 0xFA immediately followed by the
  tick whose predicted input time is the downbeat `k_d + Q`, sent `offset` early
- output tick count between start messages stays consistent (no gaps/duplicates)
- stop cancels a pending start
- offset +4000 ms with 1-bar quantum at 60 BPM (bar = 4000 ms): start is scheduled at
  the first boundary whose send time is still in the future (here the second bar)
- offset −4000 ms: start is scheduled at the first boundary (n = 0) and sent late
- outputs with forward_transport = false never receive 0xFA/0xFC

Acceptance: pressing resync in the BPM app → after at most one quantum, Resolume's
beat 1 and the sequencers' step 1 appear on the projector together with the music's
"1".

---

## Prompt 6 – Hot-plug, robustness, polish

- Port watcher thread per §7 (2 s polling, missing/reconnect, name matching with
  `N- ` prefix stripping), status column in the outputs table.
- Changing virtual port names in settings recreates the ports safely.
- All FFI/MIDI errors logged, never panic in runtime paths; review for `unwrap()`.
- Graceful shutdown: stop engine thread, close ports, restore timer resolution,
  save config.
- Log panel (last 200 lines, timestamped).
- README.md: prerequisites (virtualMIDI driver via loopMIDI or SDK, Developer notes,
  licence note), setup in the BPM app / Resolume / sequencers, how to measure and set
  the offset.

Acceptance: unplugging and replugging the Beatstep Pro during playback reconnects it
automatically with its settings; app closes cleanly.

---

## Prompt 7 – Validation tools

- Add a "Measure" panel: pick any MIDI input port via `midir` (e.g. a hardware port
  looped back, or a second virtual port for testing) and show the measured lead of
  that port's clock relative to the incoming clock (mean and p99, in ms).
- Add a stress test (`cargo test --release -- --ignored`) running the engine with the
  simulator for 10 minutes of simulated time at varying tempo and jitter; assert no
  duplicate ticks, no intervals < 0.5·P, and mean lead within 1 ms of the offset.
- Final pass: `cargo fmt`, `cargo build --release`,
  `cargo clippy --all-targets -- -D warnings`, `cargo test`.
- Do not assert real wall-clock timing in normal `cargo test`; the Measure panel is the
  place for real timing validation.
