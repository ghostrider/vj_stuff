//! Built-in clock simulator (ARCHITECTURE.md §10): an internal clock source with
//! configurable BPM and Gaussian jitter, for development without hardware. Feeds
//! the engine exactly like a real input, via the same `InputEvent` channel.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crossbeam_channel::Sender;

use crate::engine::{InputEvent, RtMsg};

/// Deterministic pseudo-random generator (splitmix64) plus a Box-Muller Gaussian
/// sampler, so jitter needs no extra dependency beyond `std`.
struct SplitMix64(u64);

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        // A zero seed would make the first output degenerate; nudge it.
        Self(seed | 1)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }

    /// Standard normal sample (mean 0, std-dev 1).
    fn next_gaussian(&mut self) -> f64 {
        let u1 = ((self.next_u64() >> 11) as f64 / (1u64 << 53) as f64).max(f64::MIN_POSITIVE);
        let u2 = (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
        (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
    }
}

/// Live-updatable simulator parameters, shared between the UI thread (which
/// writes them as the user edits settings) and the simulator thread (which
/// reads them once per tick).
#[derive(Debug)]
pub(crate) struct SharedParams {
    bpm_bits: AtomicU64,
    jitter_ms_bits: AtomicU64,
}

impl SharedParams {
    pub(crate) fn new(bpm: f64, jitter_ms: f64) -> Self {
        Self {
            bpm_bits: AtomicU64::new(bpm.to_bits()),
            jitter_ms_bits: AtomicU64::new(jitter_ms.to_bits()),
        }
    }

    fn bpm(&self) -> f64 {
        f64::from_bits(self.bpm_bits.load(Ordering::Relaxed))
    }

    fn jitter_ms(&self) -> f64 {
        f64::from_bits(self.jitter_ms_bits.load(Ordering::Relaxed))
    }

    pub(crate) fn set(&self, bpm: f64, jitter_ms: f64) {
        self.bpm_bits.store(bpm.to_bits(), Ordering::Relaxed);
        self.jitter_ms_bits
            .store(jitter_ms.to_bits(), Ordering::Relaxed);
    }
}

/// Pure tick generator: given the configured BPM/jitter, computes when the next
/// simulated clock tick should occur. Kept separate from thread/pacing concerns
/// so it can be unit-tested without real waiting — also reused directly (no
/// thread, no real time) by the Prompt 7 simulated-time stress test in
/// `engine::mod::tests`.
pub(crate) struct Simulator {
    params: Arc<SharedParams>,
    epoch: Instant,
    tick_index: u64,
    rng: SplitMix64,
}

impl Simulator {
    pub(crate) fn new(params: Arc<SharedParams>, epoch: Instant, seed: u64) -> Self {
        Self {
            params,
            epoch,
            tick_index: 0,
            rng: SplitMix64::new(seed),
        }
    }

    /// The next tick's ideal (jitter-free) instant, at the BPM in effect *now*.
    fn ideal_next_tick(&self) -> Instant {
        let period_secs = 60.0 / (self.params.bpm().max(1.0) * 24.0);
        self.epoch + Duration::from_secs_f64(self.tick_index as f64 * period_secs)
    }

    /// Advances to the next tick and returns its (possibly jittered) event.
    pub(crate) fn next_tick(&mut self) -> InputEvent {
        let ideal = self.ideal_next_tick();
        self.tick_index += 1;

        let jitter_ms = self.params.jitter_ms();
        let at = if jitter_ms > 0.0 {
            let offset_secs = self.rng.next_gaussian() * jitter_ms / 1000.0;
            if offset_secs >= 0.0 {
                ideal + Duration::from_secs_f64(offset_secs)
            } else {
                ideal - Duration::from_secs_f64(-offset_secs)
            }
        } else {
            ideal
        };

        InputEvent {
            at,
            msg: RtMsg::Clock,
        }
    }
}

/// Owns the background thread that paces simulated ticks in real wall-clock time
/// and sends them into `sender`, exactly like a real input source. Dropping this
/// stops the thread.
pub struct SimulatorHandle {
    params: Arc<SharedParams>,
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

impl SimulatorHandle {
    pub fn spawn(bpm: f64, jitter_ms: f64, sender: Sender<InputEvent>) -> Self {
        let params = Arc::new(SharedParams::new(bpm, jitter_ms));
        let stop = Arc::new(AtomicBool::new(false));

        let thread_params = Arc::clone(&params);
        let thread_stop = Arc::clone(&stop);
        let seed = Instant::now().elapsed().as_nanos() as u64 ^ 0xD1CE_5EED;
        let join = std::thread::Builder::new()
            .name("simulator".into())
            .spawn(move || run(thread_params, thread_stop, sender, seed))
            .expect("failed to spawn simulator thread");

        Self {
            params,
            stop,
            join: Some(join),
        }
    }

    /// Updates BPM/jitter live, picked up by the running thread on its next tick.
    pub fn set_params(&self, bpm: f64, jitter_ms: f64) {
        self.params.set(bpm, jitter_ms);
    }
}

impl Drop for SimulatorHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

fn run(params: Arc<SharedParams>, stop: Arc<AtomicBool>, sender: Sender<InputEvent>, seed: u64) {
    let mut sim = Simulator::new(params, Instant::now(), seed);

    while !stop.load(Ordering::Relaxed) {
        let event = sim.next_tick();

        let now = Instant::now();
        if event.at > now {
            spin_sleep::sleep(event.at - now);
        }
        if stop.load(Ordering::Relaxed) {
            break;
        }
        if sender.send(event).is_err() {
            break; // Nothing is listening anymore.
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `Duration::from_secs_f64` rounds to the nearest nanosecond, and the
    /// simulator recomputes each tick's ideal time from scratch (`index * period`)
    /// rather than accumulating, so consecutive gaps can differ from a
    /// separately-computed `period` by up to a couple of nanoseconds.
    fn assert_duration_close(actual: Duration, expected: Duration) {
        let diff = actual.abs_diff(expected);
        assert!(
            diff <= Duration::from_nanos(5),
            "expected ~{expected:?}, got {actual:?} (diff {diff:?})"
        );
    }

    #[test]
    fn ticks_are_evenly_spaced_at_the_configured_bpm_without_jitter() {
        let params = Arc::new(SharedParams::new(120.0, 0.0));
        let mut sim = Simulator::new(params, Instant::now(), 1);

        let a = sim.next_tick();
        let b = sim.next_tick();
        let c = sim.next_tick();

        let period = Duration::from_secs_f64(60.0 / (120.0 * 24.0));
        assert_duration_close(b.at - a.at, period);
        assert_duration_close(c.at - b.at, period);
    }

    #[test]
    fn jitter_perturbs_tick_times_but_stays_bounded() {
        let jitter_ms = 5.0;
        let params = Arc::new(SharedParams::new(120.0, jitter_ms));
        let mut sim = Simulator::new(params, Instant::now(), 2);

        let period = Duration::from_secs_f64(60.0 / (120.0 * 24.0));
        // Generous sanity bound (8 std-devs): not a strict physical limit, since
        // Gaussian jitter has unbounded tails, just a smoke check that nothing is
        // off by orders of magnitude (e.g. seconds instead of milliseconds).
        let sanity_bound = Duration::from_secs_f64(jitter_ms / 1000.0 * 8.0);
        for k in 1..200u64 {
            let ideal = sim.epoch + period * k as u32;
            let event = sim.next_tick();
            let delta = if event.at >= ideal {
                event.at - ideal
            } else {
                ideal - event.at
            };
            assert!(
                delta < sanity_bound,
                "jitter delta {delta:?} exceeds sanity bound"
            );
        }
    }

    #[test]
    fn set_params_takes_effect_on_the_next_tick() {
        let params = Arc::new(SharedParams::new(120.0, 0.0));
        let mut sim = Simulator::new(Arc::clone(&params), Instant::now(), 3);

        let _ = sim.next_tick();
        params.set(60.0, 0.0);
        let before = sim.next_tick();
        let after = sim.next_tick();

        let new_period = Duration::from_secs_f64(60.0 / (60.0 * 24.0));
        assert_duration_close(after.at - before.at, new_period);
    }
}
