---
name: midi-clock-protocol
description: Reference for the MIDI Beat Clock protocol as used in this app — use whenever parsing, generating, shifting, or debugging MIDI clock messages, tempo/PPQN math, or start/stop/continue handling.
---

# MIDI Clock Protocol

MIDI Beat Clock is a system real-time message used to sync tempo across devices. This app receives a clock, predicts it, and re-emits it with a **phase offset** (normally ahead of time, to compensate video latency). Getting the protocol exactly right matters more than anywhere else in the codebase.

The design decisions (what "shift" means, how Start is re-timed) are defined in `ARCHITECTURE.md` §5. This skill explains the protocol underneath; if the two ever disagree, `ARCHITECTURE.md` wins.

MTC (MIDI Time Code, quarter-frame `0xF1`) is **out of scope** for this app.

## Real-time status bytes (single byte, no data bytes, can appear mid-stream)

| Byte  | Name           | Meaning | Handling in this app |
|-------|----------------|---------|----------------------|
| 0xF8  | Clock          | Sent 24 times per quarter note (24 PPQN). The heartbeat of tempo. | Feeds the estimator |
| 0xFA  | Start          | Start from the beginning; the **next** 0xF8 is beat 1. | Re-timed to the next quantum boundary (§5.4) |
| 0xFB  | Continue       | Resume from the current position. | Ignored by default (setting) |
| 0xFC  | Stop           | Stop playback. | Forwarded immediately; output ticks pause until next Start |
| 0xFE  | Active Sensing | Optional keep-alive, not tempo-relevant. | Ignored; must never be counted as clock |

Define all of these as named constants (`const CLOCK: u8 = 0xF8;` …).

Real-time bytes can be interleaved *inside* other MIDI messages (e.g. between the status and data bytes of a Note On). When reading from a hardware input via `midir`, filter real-time bytes per byte and ignore everything else, without letting them disturb any other parsing state. The virtualMIDI input port is created with RX parsing enabled, so it delivers complete messages; the same per-byte filter still applies.

## Timing math

- 24 clocks per quarter note (PPQN = 24) is the MIDI clock standard — do not confuse with MIDI file PPQN (typically 96–960), which is a different unit.
- Microseconds per clock = `(60_000_000 / bpm) / 24` (125 BPM → 20 000 µs; 200 BPM → 12 500 µs).
- BPM from tick period = `60_000_000 / (period_us * 24)`.
- One 4/4 bar = 96 clocks; the resync quantum is 1, 2 or 4 bars = 96, 192 or 384 clocks.
- Tempo and phase are **not** taken from a single interval. The estimator (`ARCHITECTURE.md` §5.2) initialises with a least-squares fit over the first 24 ticks, then tracks with a low-gain PLL and rejects outliers (> 0.5 × period). A single late OS scheduling tick must barely move the model.

## Shifting semantics (core feature of this app)

- **Phase offset — the only shift this app implements.** Emitted clocks are offset by a fixed duration relative to the *predicted* input, without changing tempo. Positive offset = earlier (lead), negative = later. Range −4000 … +4000 ms, i.e. one full 4/4 bar at 60 BPM.
- **A lead cannot be produced by delaying or buffering.** The information is not available yet. The app therefore models the input (`T(k) = t_ref + (k − k_ref) · P`) and sends tick `k` at `T(k) − offset − trim`. Delaying by "one period minus offset" is explicitly rejected: it reacts a full beat/bar/phrase late to tempo changes and resyncs.
- Each output tick index is sent exactly once, in order. Downstream beat counters assume exactly 24 clocks per beat, so never drop or duplicate clocks; corrections happen by moving send times (with slewing), not by changing the tick count.
- **Tempo scaling (half/double time) is out of scope.** Do not implement it unless `ARCHITECTURE.md` is changed first.
- **Start is re-timed, Stop is passed through** (decision in `ARCHITECTURE.md` §5.4). The incoming Start marks the downbeat; the output Start is sent immediately before the tick of the first quantum boundary whose send time is still in the future. Output ticks keep running meanwhile. Stop cannot be sent early and is forwarded immediately.
- Start/Stop forwarding can be disabled per output; such outputs receive only 0xF8.

## Common pitfalls

- Treating clock messages as if they carry a payload — they don't; all timing comes from *when* the byte arrives. Take the timestamp first thing in the callback.
- Forwarding Start immediately on an output that runs ahead: the receiver would count the next (already advanced) tick as beat 1 and stay one offset late forever.
- Losing sync after Stop/Start because position counting wasn't reset correctly. Continue does not replay missed clocks (and is ignored by default here).
- Using relative `sleep` for output timing without an absolute schedule — errors accumulate. See [[timing-and-jitter]].
