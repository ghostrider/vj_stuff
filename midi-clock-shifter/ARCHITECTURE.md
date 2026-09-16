# midi_clock_shifter – Architecture

This document is the **authoritative design** of the app (folder `midi-clock-shifter/`,
crate `midi_clock_shifter`). The skills in `.claude/skills/` describe *how* to implement
it (protocol details, timing rules, conventions). If a skill and this document disagree,
this document wins – stop and ask the user before deviating.

A Windows desktop application (Rust, egui) that receives a MIDI clock, predicts it,
and re-emits it **ahead of time** to several MIDI destinations. Purpose: compensate
the video latency (~200 ms) between Resolume and the projector, so visuals and
clip triggers appear on screen in sync with the music.

## 1. Context

```
BPM detection app ──clock + start──▶ [virtual port "Clock Shifter In"]
                                              │
                                     midi_clock_shifter
                                              │  (clock predicted, sent early)
            ┌─────────────────────────────────┼──────────────────────────┐
            ▼                                 ▼                          ▼
[virtual port "Clock Shifter Out"]   Beatstep Pro (USB)          KeyStep / other seq. (USB)
            │                                 │                          │
            ▼                                 └── notes ──▶ Resolume ◀───┘
   Resolume (BPM sync via MIDI clock)             (clip triggers, read directly by Resolume)
```

- The **BPM detection app** is external. It can send its clock to exactly one MIDI
  device, and lists all MIDI devices present. midi_clock_shifter therefore creates its own
  virtual MIDI port, which that app can select.
- The **sequencers do not make sound**. They trigger Resolume clips via MIDI notes.
  They belong to the video chain and need the **same lead** as Resolume.
- The input source is selectable (single selection): the app's own virtual port
  `Clock Shifter In` (default), any hardware MIDI input, or the built-in simulator.
- midi_clock_shifter only opens the hardware sequencers as **outputs**. Resolume reads
  their notes directly from their USB ports.

## 2. Key decisions

| Topic | Decision |
|---|---|
| Language / UI | Rust, `eframe`/`egui`, single `.exe` |
| Virtual ports | Created by the app itself via the **virtualMIDI SDK** (Tobias Erichsen, the driver behind loopMIDI), loaded at runtime via `libloading` |
| Hardware ports | `midir` (WinMM backend): outputs, plus inputs when a hardware device is selected as input source |
| Shift semantics | **Phase offset only.** No tempo scaling, no half/double time. Start/Stop are re-timed or forwarded as defined in §5.4 |
| Shift method | **Prediction**, not delay. The app models the incoming clock and sends each tick `offset` ms before its predicted arrival time |
| Time base | `std::time::Instant` (QPC on Windows) |
| Timing | Dedicated scheduler thread, `timeBeginPeriod(1)`, `spin_sleep`, elevated thread priority |
| Config | JSON in `%APPDATA%\midi_clock_shifter\config.json` (`directories` + `serde`) |
| CLI | `clap` (derive), only a few flags (e.g. `--simulate <bpm>`, `--config <path>`) |
| Errors | `anyhow` in `main.rs`/UI, `thiserror` for typed errors in `engine/`, `vmidi/`, `output.rs` |
| Target | Windows 10/11 x64 only |

### Prerequisite: virtualMIDI driver
The virtualMIDI kernel driver must be installed (it is installed together with
loopMIDI, or via the virtualMIDI SDK). The app loads `teVirtualMIDI64.dll` at startup.
If the DLL or driver is missing, the app must start anyway, show a clear error banner
and keep all other features usable (hardware outputs still work, but there is no input).

The licence terms of the virtualMIDI SDK must be checked by the user before any
distribution. This is a personal tool; do not bundle the DLL in the repo.

## 3. Modules

```
src/
├── main.rs            – startup, thread wiring, eframe launch
├── app.rs             – egui UI (reads snapshots, sends commands)
├── config.rs          – AppConfig, load/save, defaults
├── vmidi/
│   ├── mod.rs         – safe wrapper: VirtualPort (create, send, close, Drop)
│   └── ffi.rs         – raw FFI declarations + libloading
├── ports.rs           – midir enumeration, hot-plug polling, filtering
├── engine/
│   ├── mod.rs
│   ├── estimator.rs   – tick period + phase model (PLL), lock state, jitter
│   ├── transport.rs   – start/stop/resync state machine, bar/phrase counting
│   └── scheduler.rs   – per-output send times, slewing, monotonic tick emission
├── output.rs          – trait ClockOutput + impls (VirtualOut, MidirOut)
├── logging.rs         – ring-buffer log::Log backend for the UI's Log panel
├── timing.rs          – timer resolution, thread priority, precise sleep
├── clock.rs           – trait TimeSource (real + simulated, for tests)
└── sim.rs             – built-in clock simulator (BPM + jitter) for testing
```

### Threads

1. **UI thread** (eframe). Never touches MIDI directly.
2. **Input callback** (driver thread, from virtualMIDI or `midir`). Does only this:
   take the timestamp *first*, map the byte to a small `Copy` enum
   (`RtMsg::{Clock, Start, Continue, Stop}`), then `try_send` an
   `InputEvent { at: Instant, msg: RtMsg }` into a **bounded, pre-allocated**
   `crossbeam_channel` (capacity e.g. 1024). No allocation, no locks, no logging.
   Overflow increments an atomic counter shown in the UI.
3. **Engine/scheduler thread** (high priority). Owns the estimator, transport state,
   and **all** output connections. Drains the input channel, updates the model,
   sends due messages, then sleeps precisely until the next due event (or at most
   ~5 ms, to process input and commands).
4. **Port watcher**. Polls `midir` port lists every 2 s and reconnects/marks
   missing outputs and the selected hardware input as needed (§7). Implemented
   as a periodic check on the **UI thread** (piggy-backing on its existing
   `request_repaint_after(33 ms)` cadence) rather than a separate OS thread:
   enumeration and reconnect construction (`MidirOut::open`/`VirtualOut::create`)
   are not on any latency-sensitive path, egui already wakes up regularly enough
   to hit the 2 s cadence, and this avoids a fourth thread's lifecycle/shutdown
   coordination for no real benefit. It still only ever talks to the engine
   through the same `Command` channel as any other UI action (`add_output`,
   `remove_output`, `set_input`), never touching engine-owned state directly.

Communication:
- UI → engine: `crossbeam_channel<Command>` (set offset, set trims, enable output,
  set resync quantum, manual resync, reload ports …).
- Engine → UI: `arc-swap` (`ArcSwap<Snapshot>`), published at most ~30×/s and only
  in the engine's idle phase, never between computing a send time and sending. The
  engine never waits on a lock held by the UI. The snapshot contains BPM, lock state,
  jitter, beat/bar/phrase and per-output status.
  The UI calls `ctx.request_repaint_after(33 ms)`.
  Log lines (last 200, timestamped) are *not* routed through the snapshot: they
  can originate on any thread (UI, engine, port watcher, input callbacks aside —
  those never log), so they go through a small global ring buffer
  (`logging.rs`) that both the installed `log::Log` backend and the UI's Log
  panel share directly.

## 4. Virtual ports (virtualMIDI SDK)

Functions used (verify exact signatures against `teVirtualMIDI.h` from the SDK
before implementing – do not guess):

- `virtualMIDICreatePortEx2(name: LPCWSTR, callback, instance: DWORD_PTR, maxSysexLength: DWORD, flags: DWORD) -> LPVM_MIDI_PORT`
- `virtualMIDISendData(port, data: LPBYTE, length: DWORD) -> BOOL`
- `virtualMIDIClosePort(port)`
- `virtualMIDIShutdown(port) -> BOOL`
- `virtualMIDIGetVersion(...)`, `virtualMIDIGetDriverVersion(...)` – shown in the UI
- Errors via `GetLastError()`

Callback: `extern "system" fn(port, data: *const u8, length: u32, instance: usize)`.
Check the header for how a closed/shut-down port is signalled (likely null data /
zero length) and handle it.

Two ports are created:

| Port | Default name | Direction (as seen by other apps) | Flags |
|---|---|---|---|
| Input | `Clock Shifter In` | appears as a MIDI **output** (the BPM app sends to it) | parse RX, instantiate RX-only |
| Output | `Clock Shifter Out` | appears as a MIDI **input** (Resolume reads from it) | parse TX, instantiate TX-only |

Verify in the header which of the `TE_VM_FLAGS_INSTANTIATE_*` flags yields each
direction; the goal is that each port is visible in only one direction to avoid
misconfiguration. Port names are configurable; changing them recreates the port.

`VirtualPort` implements `Drop` (closes the port). The callback instance pointer must
point to data that outlives the port (e.g. a leaked/boxed `Sender`, freed after close).

## 5. Clock engine

### 5.1 Input messages
Only realtime messages are relevant:
`0xF8` clock, `0xFA` start, `0xFB` continue, `0xFC` stop. Everything else is ignored,
including `0xFE` active sensing (counted in a debug counter). `0xFB` is ignored by
default. With a `midir` input, real-time bytes may be interleaved inside other
messages; filter them per byte without disturbing anything else.

### 5.2 Estimator (estimator.rs)
Model of the incoming tick stream: `T(k) = t_ref + (k − k_ref) · P`
where `k` is the input tick index and `P` the tick period (24 PPQN).

- On each tick: `predicted = T(k)`, `err = actual − predicted`.
- Outlier rejection: if `|err| > 0.5 · P`, count as outlier; after 3 consecutive
  outliers, re-initialise the model (tempo jump or dropout).
- PLL update (2nd order): `phase += α·err`, `P += β·err` with small gains
  (start with α = 0.1, β = 0.002; make them constants, tune later).
  Clamp `P` to 30–300 BPM.
- Initial estimate: least-squares fit over the first 24 ticks.
- **Lock state:** `NoSignal` → `Acquiring` (fewer than 24 ticks) → `Locked`
  (jitter below threshold) → `NoSignal` after no tick for `3·P` (min 250 ms).
- Jitter: running RMS of `err` over the last 96 ticks, shown in ms.
- BPM displayed = `60 / (24 · P)`, smoothed for display only.

The estimator is pure logic with an injected time source: fully unit-testable.

### 5.3 Scheduler (scheduler.rs)
The model defines when input tick `k` *will* arrive. Output tick `k` for output `o`
is sent at:

```
send_time(o, k) = T(k) − effective_offset − trim(o)
```

`effective_offset` is the global offset (default +200 ms, range −4000 … +4000 ms).
The range covers one full 4/4 bar at 60 BPM (4 × 1000 ms), so at slow tempos every
position within a bar can be reached in both directions; at faster tempos it spans
more than one bar. A lead of 4 s is still well within what the model can predict
(a 0.05 BPM tempo error at 60 BPM gives about 3 ms error at 4 s).
**Sign convention: positive = earlier (lead), negative = later (delay).** A negative
offset needs no special handling – the scheduler simply sends later than predicted.
`trim(o)` is a per-output fine correction (range −50 … +50 ms, default 0).

Rules:
- Output tick indices are **strictly monotonic** per output. A tick index is never
  sent twice and never skipped silently.
- Because offset > tick period, sending requires prediction several ticks ahead;
  always use the latest model.
- **Catch-up:** if the model moves so that the next tick's send time is already in the
  past, send it now, but never two ticks closer than `0.5 · P` apart.
- **Hold-back:** if the model moves the other way, simply wait.
- **Slewing:** changes of offset or trim are applied gradually, max 2 ms per tick,
  so receivers do not see a tempo jump.
- When the lock state is `NoSignal`, stop sending ticks (do not send `0xFC`
  automatically).
- Output order: all due messages for the same instant are sent in one pass;
  `0xFA` is always sent immediately before the tick it belongs to.

### 5.4 Transport (transport.rs)
MIDI semantics: after `0xFA`, the receiver treats the **next** `0xF8` as beat 1.
The shifter keeps its own position counter: `tick_in_bar` (0–95, 4/4 at 24 PPQN)
and `bar_in_phrase` (0 … quantum−1).

Resync quantum (configurable): 1, 2 or 4 bars (default 1).
`Q = 96 · quantum` ticks.

On incoming `0xFA` at input tick index `k_s` (the next incoming `0xF8` is the new
downbeat, index `k_d`):
1. Set the position counter so that input tick `k_d` is tick 0 of bar 0.
2. The downbeat itself is already past for all outputs with a lead. Therefore
   schedule an output `0xFA` for each output immediately before output tick
   `k_d + n·Q`, where `n ≥ 1` is the **smallest** value for which
   `send_time(o, k_d + n·Q)` is still in the future. With large offsets
   (e.g. 4000 ms lead at 1-bar quantum and 60 BPM, where one bar is exactly 4000 ms)
   this automatically moves to a later boundary.
   Outputs with zero or negative offset use `n = 0` if still possible.
3. Output ticks keep running continuously in the meantime (no gap, no restart).
4. If a second `0xFA` arrives before the pending one was sent, it replaces it.

If the clock was not running before (state `NoSignal`/`Acquiring`), wait until
`Locked`, then schedule `0xFA` before the next downbeat boundary (multiple of `Q`
counted from `k_d`) and start sending ticks just before that `0xFA`.

On incoming `0xFC`: forward immediately to all outputs that have "forward start/stop"
enabled, cancel pending starts, stop sending ticks until the next `0xFA`.
(A stop cannot be sent early – accepted.)

Manual resync button in the UI: behaves like an incoming `0xFA` at the next
incoming tick.

Per output option: **forward start/stop** (default on). If off, the output only
receives `0xF8`.

## 6. Outputs (output.rs)

```rust
pub trait ClockOutput: Send {
    fn name(&self) -> &str;
    fn send(&mut self, msg: &[u8]) -> Result<()>;
    fn is_connected(&self) -> bool;
}
```
Implementations: `VirtualOut` (the `Clock Shifter Out` port) and `MidirOut`
(hardware). Keep the trait small so a Windows MIDI Services backend can be added
later without touching the scheduler.

Each output has settings: `enabled`, `trim_ms`, `forward_transport`.
Settings are stored **by port name**, never by index.

## 7. Ports and hot-plug (ports.rs)
- List all `midir` output ports every 2 s.
- **Loop protection:** hide the app's own virtual ports from the input and output
  lists as appropriate (match by configured names): `Clock Shifter In` never appears
  as an output, `Clock Shifter Out` never as an input.
- If the selected hardware input and an enabled output belong to the same device,
  show a warning (the device might echo the clock via MIDI thru).
- Enabled port disappears → status `Missing` (grey), connection dropped.
- It reappears → reconnect automatically, keep settings, log it.
- Windows may append indices to duplicate device names (e.g. `2- Beatstep Pro`).
  Match by exact name first, then by name with a leading `N- ` prefix stripped.

## 8. Timing (timing.rs)
- `timeBeginPeriod(1)` at start, `timeEndPeriod(1)` on exit (RAII guard).
- Scheduler thread priority: `THREAD_PRIORITY_TIME_CRITICAL` (via `windows` crate).
- Precise sleep: `spin_sleep` (sleep to ~1 ms before, then spin).
- Target: scheduled vs. actual send time error < 1 ms at the 99th percentile.
  The engine measures this and shows it in the UI ("send jitter").

## 9. UI (app.rs)
Single window, wide layout, dark theme. Layout: **one toolbar and two panels**.
MIDI devices are detected automatically (see §7).

- **Theme:** forced dark (`egui::ThemePreference::Dark`, set on the `CreationContext` in
  `main.rs`), regardless of the OS theme setting.
- **Window size:** default `1440×770`, resizable; wide enough that the left input panel,
  the right outputs table and the toolbar controls are all visible without scrolling.

- **Toolbar:** global offset (slider −4000 … +4000 ms plus numeric field, default
  +200 ms, positive = earlier; the slider needs fine resolution around the typical
  0–500 ms, e.g. a logarithmic/non-linear scale, and the numeric field accepts exact
  values; next to it the offset is shown in beats at the current tempo, e.g.
  "= 0.42 beats"), resync quantum dropdown (1/2/4 bars), big "Resync"
  button with pending-start indicator, lock state (colour), BPM in, input jitter,
  send jitter, virtualMIDI driver status (version or error banner below the toolbar).
- **Left panel – input source (single selection, radio buttons):**
  `Clock Shifter In` (virtual, default, first row) · all hardware MIDI inputs ·
  Simulator. Missing devices shown grey.
- **Beat display** (top of the right panel or in the toolbar): 4 beat cells + bar counter within the phrase; beat 1 flashes
  strongly. Show the **output** (shifted) position; optional small row for input
  position.
- **Right panel – outputs (multiple selection):** one row per port: checkbox enabled · name · status
  (connected/missing/error) · trim slider ±50 ms · checkbox forward start/stop.
  `Clock Shifter Out` is always the first row.
- **Settings (collapsible):** virtual port names, continue handling, simulator.
- **Log:** last 200 lines, timestamped.

Config is saved on every change (debounced 500 ms) and on exit.

## 10. Simulator (sim.rs)
For development without hardware: an internal clock source with configurable BPM,
Gaussian jitter (ms), optional tempo ramps and a "send start" button. It feeds the
engine exactly like the virtual input. Enabled via UI setting or `--simulate <bpm>`.

## 10a. Measure panel (validation, PROMPTS.md Prompt 7)

A collapsible UI section (central panel, alongside Settings/Log) for validating
real-world lead outside of `cargo test` (timing-and-jitter skill: "Real-world
validation happens in the app's Measure panel"). The user picks any `midir` input
port (`ports::list_all_input_ports` — unlike the main input's port list, this one
deliberately does *not* exclude `Clock Shifter Out`, so it can be looped straight
back with no external cable; a physically-looped hardware port also works). Each
`0xF8` arriving there is correlated with the *closest* predicted main-input tick
(`TickModel::nearest_tick_index`, since no index travels in the byte stream itself),
and `predicted − actual` is recorded as that tick's measured lead. The engine
publishes a running mean and p99 (last 200 samples) in the `Snapshot`; a well-tuned
system's measured lead should sit close to the configured offset.

Wired like a second, independent input: its own `Command::SetMeasureInput`/
`ClearMeasureInput`, its own connection lifecycle in `app.rs` (`apply_measure_input`,
hooked into the same hot-plug `reconcile_ports` poll as the main input/outputs), but
it never feeds into the estimator or scheduler — it only reads the engine's already-
published model, so a bad measure port can't disturb the actual clock pipeline.

## 11. Dependencies

```toml
eframe = "0.*"            # pin to current version when creating the project
midir = "0.*"
libloading = "0.*"
spin_sleep = "1"
crossbeam-channel = "0.5"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
directories = "5"
anyhow = "1"
thiserror = "1"
clap = { version = "4", features = ["derive"] }
arc-swap = "1"
log = "0.4"
windows = { version = "0.*", features = ["Win32_Media", "Win32_System_Threading", "Win32_Foundation"] }
```
Use current versions when creating the project. Keep the dependency list small.

## 12. Quality bar
- `cargo fmt`, `cargo build --release` and `cargo clippy --all-targets -- -D warnings`
  clean after every step. Fix warnings instead of `#[allow]`, unless a comment explains why.
- Protocol values as named constants (`const CLOCK: u8 = 0xF8;` …), no bare hex in logic.
- No async runtime; all timing on plain OS threads.
- Unit tests for estimator, scheduler and transport using the simulated time source.
- No `unwrap()` on MIDI or FFI results in runtime paths; errors go to the log.
- No allocation, blocking locks or logging in input callbacks or between computing a
  send time and sending (the hot path).
