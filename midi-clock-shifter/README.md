# midi_clock_shifter

A Windows desktop app that receives MIDI clock, predicts it, and re-emits it with a
**phase offset** — normally ahead of time — to one or more MIDI outputs. It exists to
compensate the video latency (typically ~200 ms) between Resolume and a projector, so
visuals and clip triggers land on screen together with the music's beat.

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

See `ARCHITECTURE.md` for the full design (signal flow, estimator/scheduler/transport
internals, UI layout) and `PROMPTS.md` for the build history.

## Features

- Predicts incoming MIDI clock with a PLL-based estimator (lock state, jitter,
  outlier rejection) instead of just relaying it, so the shifted output stays smooth
  through normal jitter and tempo drift.
- Phase offset −4000…+4000 ms (positive = earlier), applied per output with an
  individual ±50 ms trim, both slewed so live changes never glitch.
- Start/stop/resync transport handling: an incoming or manual "Resync" re-times the
  downbeat to the next 1/2/4-bar boundary without interrupting the running clock.
- Multiple simultaneous outputs (the app's own virtual `Clock Shifter Out` plus any
  number of hardware MIDI ports), each independently enabled, trimmed, and with its
  own forward-start/stop setting.
- Automatic hot-plug reconnect for outputs/inputs that disappear and come back.
- A **Measure** panel and a **Log** panel for validating and debugging real-world
  behavior (see below).

## Prerequisites

- **Windows 10/11 x64.** This app is not portable to other platforms — it uses the
  Windows timer APIs (`timeBeginPeriod`, `SetThreadPriority`) directly.
- **Rust** (stable toolchain) to build from source — see [Building](#building).
- **The virtualMIDI driver and DLL** — see the next section. Without it, the app
  still runs (hardware MIDI inputs/outputs keep working), but it can't create its own
  `Clock Shifter In`/`Clock Shifter Out` virtual ports.

## Getting and installing the virtualMIDI DLL

midi_clock_shifter creates its own virtual MIDI ports using Tobias Erichsen's
**virtualMIDI** driver — the same underlying driver used by the popular loopMIDI
utility. The app talks to it by loading `teVirtualMIDI64.dll` at runtime (via
`libloading`, see `src/vmidi/ffi.rs`); it is **not** bundled with this repository and
must never be committed to it (see [Licence](#licence) below) — you install it once,
yourself.

There are two ways to get the DLL onto your machine. **Option A is recommended** for
normal use; Option B is only useful if you specifically don't want to run an
installer (e.g. a portable/no-install setup).

### Option A — Install loopMIDI (recommended)

1. Download and run the loopMIDI installer from
   <https://www.tobias-erichsen.de/software/loopmidi.html>.
2. Complete the installation (it installs the virtualMIDI kernel driver as a
   dependency, and drops `teVirtualMIDI64.dll` in a location Windows finds
   automatically — you don't need to touch the DLL yourself).
3. You can leave loopMIDI itself closed/uninstalled from your startup items —
   midi_clock_shifter only needs the driver + DLL it installed, not the loopMIDI
   application running.
4. Restart midi_clock_shifter (or start it for the first time) after installing.

### Option B — Install the virtualMIDI SDK and place the DLL manually

1. Download the virtualMIDI SDK from
   <https://www.tobias-erichsen.de/software/virtualmidi.html>.
2. Extract the SDK zip. Inside, you'll find (paths vary slightly by SDK version):
   - `teVirtualMIDI.h` — the C header (only needed if you're changing the FFI layer
     in `src/vmidi/ffi.rs`; not needed to just run the app).
   - An installer for the virtualMIDI **kernel driver** — run it first. The DLL alone
     does nothing without this driver installed.
   - `x64/teVirtualMIDI64.dll` (and `x86/teVirtualMIDI32.dll`, which this app doesn't
     use — it's a 64-bit-only app).
3. Run the driver installer from the SDK package.
4. Copy `teVirtualMIDI64.dll` next to the built executable:
   ```
   target\release\midi_clock_shifter.exe
   target\release\teVirtualMIDI64.dll   <- place it here
   ```
   Windows' DLL search order checks the executable's own folder before the system
   directories, so this works without any system-wide install of the DLL itself
   (the kernel driver from step 3 still has to be installed system-wide, though —
   that part isn't optional).
5. Restart midi_clock_shifter after copying the DLL.

### Verifying it worked

- The toolbar area, just under the main controls, shows either:
  - `virtualMIDI dll <version> / driver <version>` in grey — it's working.
  - a **red banner** reading `virtualMIDI unavailable: ...` — the DLL or driver
    wasn't found; re-check the steps above, and check the **Log** panel at the
    bottom of the window for the exact error.
- The **virtualMIDI debug** section (central panel) shows live Clock/Start/
  Continue/Stop counters for `Clock Shifter In` — these only move once something is
  actually sending to that port, which requires the DLL/driver to be working first.
- In Windows' MIDI device lists (e.g. in Pulse's or Resolume's MIDI settings), you
  should see `Clock Shifter In` and `Clock Shifter Out` appear **only while
  midi_clock_shifter is running** — they're created and destroyed with the app.

### Licence

The virtualMIDI SDK/driver has its own licence terms from Tobias Erichsen — read them
on the pages linked above before distributing anything built against it. This is a
personal tool: `teVirtualMIDI64.dll` is never committed to this repository, and you
install/obtain it yourself per the steps above.

## Building

```sh
cargo build --release
```

The binary is a single `.exe` (`target/release/midi_clock_shifter.exe`); no installer,
no bundled DLLs (see above for the one DLL you need to provide yourself).

## Project layout

```
src/
├── main.rs            # CLI args, logger/timer setup, eframe launch
├── app.rs              # egui UI: toolbar, input/output panels, Measure, Settings, Log
├── config.rs            # AppConfig, load/save JSON
├── vmidi/               # virtualMIDI SDK bindings (ffi.rs) + safe VirtualPort wrapper
├── ports.rs             # midir enumeration, hot-plug reconnect, name matching
├── engine/
│   ├── estimator.rs      # tempo/phase model (PLL), lock state
│   ├── transport.rs      # start/stop/resync boundary math
│   └── scheduler.rs      # per-output send times, slewing, monotonic emission
├── output.rs            # ClockOutput trait + VirtualOut / MidirOut
├── logging.rs           # ring-buffer log backend for the Log panel
├── timing.rs            # timer resolution, thread priority, precise sleep
├── clock.rs             # TimeSource trait (real + simulated, for tests)
└── sim.rs               # built-in clock simulator (BPM + jitter)
```

## Setup

1. **Start midi_clock_shifter first.** It creates `Clock Shifter In` and
   `Clock Shifter Out` as soon as it starts (as long as the virtualMIDI driver is
   installed — see above), and other apps only see them while it's running.
2. **BPM detection app (e.g. Pulse):** set its MIDI clock output to `Clock Shifter In`.
3. **Resolume:** in Preferences → MIDI, set the clock input to `Clock Shifter Out`,
   and enable that output's row in midi_clock_shifter's right-hand panel.
4. **Hardware sequencers** (Beatstep Pro, KeyStep, …) that trigger Resolume clips:
   connect them over USB, then enable their row in the outputs panel too. They need
   the same lead as Resolume, since they're driving clips through their own MIDI-note
   connection to Resolume, separate from this app.
5. Leave the input source on `Clock Shifter In` (the default, left panel) unless
   you're feeding clock from a hardware device directly, or testing with the
   built-in **Simulator**.

If an app doesn't seem to receive anything from `Clock Shifter In`/`Out`: MIDI
devices are typically enumerated once when the *other* app starts, so if you restart
midi_clock_shifter, you may need to reselect the port (or restart the other app) so it
opens a fresh handle to the recreated port — the port name looking the same in a
dropdown doesn't guarantee the handle underneath is still the same instance.

## Measuring and setting the offset

The **offset** (toolbar, −4000…+4000 ms, positive = earlier) is how far ahead of the
predicted beat every output tick is sent. In practice:

1. Set up your actual video chain (camera on the projected output, or by eye) and
   play a steady beat.
2. Start with the default (+200 ms) and watch whether Resolume's/the projector's
   visual beat lands before or after the audible beat.
3. Increase the offset if visuals lag behind the audio; decrease (or go negative) if
   they lead. The offset is also shown converted to beats at the current BPM
   (e.g. "= 0.42 beats") to help judge how large a correction you're making.
4. Per-output **trim** (±50 ms, in the outputs panel) fine-tunes an individual output
   relative to the global offset — useful if one sequencer's own MIDI-to-clip latency
   differs slightly from Resolume's.
5. Changes are slewed (max 2 ms per tick) rather than jumping, so retuning while
   everything is running never produces an audible/visual glitch.

Use the **Resync** button (or an incoming Start from the BPM app) to force Resolume's
and the sequencers' beat 1 back in phase with the music — it takes effect at the next
resync-quantum boundary (1/2/4 bars, toolbar dropdown), not instantly, since the
output stream may already be running ahead of the real downbeat because of the lead.

## Measure panel

The collapsible **Measure** section (central panel) validates the *actual* lead being
produced, independent of what the app assumes internally. Pick any MIDI input port:

- **`Clock Shifter Out` looped straight back to itself** — no cable needed, since it
  simply appears in the same port dropdown as any other MIDI input. This is the
  easiest self-check.
- Or a real hardware port, physically MIDI-looped from an enabled output.

Every `0xF8` arriving on the selected port is matched against the closest predicted
input tick, and the residual (predicted − actual) is shown as **mean** and **p99**
lead, in ms. In a correctly working setup this should sit close to the configured
offset; a large or drifting discrepancy points to a real driver/USB/OS latency issue
rather than anything this app's model can fix.

## Troubleshooting

- **Toolbar shows "NoSignal"/"Acquiring" and never "Locked":** check the selected
  input source (left panel) and that the sending app is actually pointed at
  `Clock Shifter In` (or the selected hardware port). The debug counters under the
  outputs panel show raw Clock/Start/Continue/Stop counts straight from the virtual
  input port, independent of lock state — if those stay at zero, the bytes aren't
  reaching the port at all (see the reselection note above, and the virtualMIDI
  section above if this is a fresh install).
- **An output shows "missing":** its port isn't currently visible to Windows (device
  unplugged, or not powered on). It reconnects automatically once it reappears,
  keeping its trim/forward-transport settings.
- **Log panel** (bottom of the window) keeps the last 200 timestamped log lines from
  every part of the app — check it first for connection/driver errors.

## Testing

```sh
cargo test --release
```

runs the full unit test suite (estimator, scheduler, transport, port-name matching,
config round-tripping, plus a few real-thread wiring/end-to-end checks) using
simulated or injected time — none of it depends on real wall-clock timing, so it's
fast and deterministic.

A separate, `#[ignore]`d stress test simulates 10 minutes of clock time (varying tempo
and jitter) in a fraction of a real second and checks tick monotonicity, minimum
spacing, and mean lead accuracy:

```sh
cargo test --release -- --ignored
```

## Command-line flags

- `--simulate <bpm>`: start with the built-in simulator selected as input source, at
  the given BPM, instead of a real input.
- `--config <path>`: use a config file at `<path>` instead of the default
  (`%APPDATA%\midi_clock_shifter\config.json`).
