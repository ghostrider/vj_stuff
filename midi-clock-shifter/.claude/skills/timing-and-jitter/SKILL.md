---
name: timing-and-jitter
description: Guidance for writing low-jitter, drift-free real-time timing loops in Rust — use whenever touching the input callbacks, the engine/scheduler thread, sleeps, or anything on the hot path between receiving a MIDI byte and emitting one.
---

# Timing and Jitter in Rust

This app's output quality is judged entirely by timing accuracy. Target: scheduled vs. actual send time error < 1 ms at the 99th percentile (`ARCHITECTURE.md` §8). Treat the receive → predict → emit path as a real-time hot path.

## The hot path in this app

- Input callbacks (virtualMIDI driver thread, `midir` input thread).
- The engine/scheduler thread between computing a send time and sending the bytes.

The UI thread, port watcher and config saving are **not** on the hot path and must never block it.

## Rules for the hot path

- **No allocation, no blocking locks, no logging.** Input callbacks take the timestamp first, map the byte to a `Copy` enum and `try_send` it into a bounded, pre-allocated `crossbeam_channel`. The engine publishes UI data via `arc-swap` in its idle phase only. A reallocating `Vec::push`, a `println!`, or a mutex contended by the UI thread can each cost more than a clock interval at fast tempos (200 BPM = 12.5 ms per clock; 300 BPM ≈ 8.3 ms).
- **Never use `std::thread::sleep` alone for sub-millisecond scheduling.** Use `spin_sleep`: sleep until ~1 ms before the deadline, then spin.
- **Always schedule against an absolute deadline**, never a repeated relative delay. In this app every deadline comes from the model: `send_time(o, k) = T(k) − offset − trim(o)`, with `T(k) = t_ref + (k − k_ref) · P`.
- **Use `std::time::Instant` for all timing** (QPC on Windows), never `SystemTime`.
- Dedicated OS thread for the engine, priority `THREAD_PRIORITY_TIME_CRITICAL`. **No async runtime** (tokio, async-std).

## Drift correction and model updates

1. The schedule is always derived from the model, never from the previously emitted tick, so rounding errors cannot compound.
2. The model is updated **on every input tick** by a low-gain 2nd-order PLL (`ARCHITECTURE.md` §5.2). The small gains are what filter noise; do **not** add a hard "only recompute above 0.5 BPM" threshold, it would cause phase to drift until the threshold trips and then jump.
3. Outliers (> 0.5 × period) are rejected; three in a row re-initialise the model (tempo jump or dropout).
4. Model or offset changes never produce jumps on the output: catch-up ticks are never closer than 0.5 × period, offset/trim changes slew at max 2 ms per tick, and no tick index is sent twice.

## Measuring jitter

- The engine always records `actual_send − scheduled_send` and input `err` into **pre-allocated** fixed-size ring buffers/counters (no allocation, no logging) and publishes p99/RMS values via the snapshot. This runs in release builds too, because the UI shows "send jitter" and "input jitter".
- Human-readable logging of timing events happens only outside the hot path (e.g. the UI formats snapshot values).

## Platform notes (Windows 10/11 x64 is the only target)

- `timeBeginPeriod(1)` at start and `timeEndPeriod(1)` on exit via an RAII guard (`windows` crate), then `spin_sleep` for final precision.
- USB MIDI input timestamps jitter by a few ms; that is what the estimator is for — do not try to "fix" it on the output side.

## Testing timing-sensitive code

- Unit-test the *math* (estimator, scheduler send times, slewing, transport/resync boundaries) with the injected `TimeSource` and the simulator (`clock.rs`, `sim.rs`). Never assert on real wall-clock timing in normal `cargo test`.
- Long-running stress tests use simulated time and are marked `#[ignore]` (`cargo test --release -- --ignored`).
- Real-world validation happens in the app's Measure panel, which records actual timestamps and reports lead and jitter statistics.
