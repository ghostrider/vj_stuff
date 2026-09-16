# midi_clock_shifter

A Windows desktop app that receives MIDI clock, predicts it, and re-emits it with a
**phase offset** — normally ahead of time — to one or more MIDI outputs. It exists to
compensate the video latency (typically ~200 ms) between Resolume and a projector, so
visuals and clip triggers land on screen together with the music's beat.

See `ARCHITECTURE.md` for the full design, and `PROMPTS.md` for the build history.

## Prerequisites

- **Windows 10/11 x64.** This app is not portable to other platforms (it uses the
  Windows timer APIs and `SetThreadPriority`).
- **A virtualMIDI driver.** midi_clock_shifter creates its own virtual MIDI ports
  (`Clock Shifter In` / `Clock Shifter Out`) using Tobias Erichsen's
  [virtualMIDI SDK](https://www.tobias-erichsen.de/software/virtualmidi.html). You get
  the driver either by:
  - installing [loopMIDI](https://www.tobias-erichsen.de/software/loopmidi.html) (it
    installs the same underlying driver), or
  - installing the virtualMIDI SDK directly.

  The app loads `teVirtualMIDI64.dll` at startup. If the driver or DLL isn't found, the
  app still starts — it shows a red banner under the toolbar, hardware inputs/outputs
  keep working, but there is no virtual `Clock Shifter In`/`Out` until the driver is
  installed and the app is restarted.
- **Licence note:** the virtualMIDI SDK has its own licence terms (see the link above).
  This repository never bundles `teVirtualMIDI64.dll` — install the SDK/loopMIDI
  yourself, and check the licence before distributing anything built against it.

## Building

```sh
cargo build --release
```

The binary is a single `.exe` (`target/release/midi_clock_shifter.exe`); no installer.

## Setup

1. **Start midi_clock_shifter first.** It creates `Clock Shifter In` and
   `Clock Shifter Out` as soon as it starts (as long as the virtualMIDI driver is
   installed), and other apps only see them while it's running.
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

## Troubleshooting

- **Toolbar shows "NoSignal"/"Acquiring" and never "Locked":** check the selected
  input source (left panel) and that the sending app is actually pointed at
  `Clock Shifter In` (or the selected hardware port). The debug counters under the
  outputs panel show raw Clock/Start/Continue/Stop counts straight from the virtual
  input port, independent of lock state — if those stay at zero, the bytes aren't
  reaching the port at all (see the reselection note above).
- **An output shows "missing":** its port isn't currently visible to Windows (device
  unplugged, or not powered on). It reconnects automatically once it reappears,
  keeping its trim/forward-transport settings.
- **Log panel** (bottom of the window) keeps the last 200 timestamped log lines from
  every part of the app — check it first for connection/driver errors.

## Command-line flags

- `--simulate <bpm>`: start with the built-in simulator selected as input source, at
  the given BPM, instead of a real input.
- `--config <path>`: use a config file at `<path>` instead of the default
  (`%APPDATA%\midi_clock_shifter\config.json`).
