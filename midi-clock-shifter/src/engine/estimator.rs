//! Tick period + phase model (PLL), lock state, jitter (ARCHITECTURE.md §5.2).
//!
//! Pure logic, no I/O: callers hand in `Instant`s (real or simulated), which is
//! what makes this fully unit-testable without real waiting.

use std::time::{Duration, Instant};

/// Ticks needed for the initial least-squares fit before tracking starts.
const INITIAL_FIT_SAMPLES: usize = 24;
/// Window size for the running jitter RMS.
const JITTER_WINDOW: usize = 96;
/// Outlier rejection threshold, as a fraction of the current period.
const OUTLIER_FACTOR: f64 = 0.5;
/// Consecutive outliers before the model is re-initialised.
const MAX_OUTLIER_STREAK: u32 = 3;
/// PLL phase gain.
const ALPHA: f64 = 0.1;
/// PLL period (frequency) gain.
const BETA: f64 = 0.002;
/// Jitter RMS below which the estimator is considered `Locked`.
const LOCK_JITTER_THRESHOLD_MS: f64 = 5.0;
/// Clamp: 30-300 BPM at 24 PPQN, expressed as a period range in seconds.
const PERIOD_MIN_SECS: f64 = 60.0 / (300.0 * 24.0);
const PERIOD_MAX_SECS: f64 = 60.0 / (30.0 * 24.0);
/// No-signal timeout floor (ARCHITECTURE.md §5.2: "after no tick for 3·P (min 250 ms)").
const NO_SIGNAL_FLOOR_SECS: f64 = 0.25;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LockState {
    #[default]
    NoSignal,
    Acquiring,
    Locked,
}

/// Fixed-size ring buffer computing a running RMS in O(1) per sample.
#[derive(Debug, Clone)]
struct JitterRms {
    buf: [f64; JITTER_WINDOW],
    len: usize,
    pos: usize,
    sum_sq: f64,
}

impl JitterRms {
    fn new() -> Self {
        Self {
            buf: [0.0; JITTER_WINDOW],
            len: 0,
            pos: 0,
            sum_sq: 0.0,
        }
    }

    fn push(&mut self, value_ms: f64) {
        let old = self.buf[self.pos];
        self.sum_sq -= old * old;
        self.buf[self.pos] = value_ms;
        self.sum_sq += value_ms * value_ms;
        self.pos = (self.pos + 1) % JITTER_WINDOW;
        self.len = (self.len + 1).min(JITTER_WINDOW);
    }

    fn rms_ms(&self) -> f64 {
        if self.len == 0 {
            0.0
        } else {
            (self.sum_sq / self.len as f64).sqrt()
        }
    }
}

/// Incrementally-accumulated sufficient statistics for an ordinary-least-squares
/// fit of `y_k ≈ intercept + slope * k`, extended by one point at a time. Unlike
/// a fixed-gain filter, its variance keeps shrinking as more points arrive.
#[derive(Debug, Clone, Copy, Default)]
struct RunningFit {
    n: f64,
    sum_x: f64,
    sum_x2: f64,
    sum_y: f64,
    sum_xy: f64,
}

impl RunningFit {
    fn new() -> Self {
        Self::default()
    }

    fn push(&mut self, x: f64, y: f64) {
        self.n += 1.0;
        self.sum_x += x;
        self.sum_x2 += x * x;
        self.sum_y += y;
        self.sum_xy += x * y;
    }

    /// Returns `(intercept, slope)`, or `None` if fewer than 2 points so far.
    fn fit(&self) -> Option<(f64, f64)> {
        if self.n < 2.0 {
            return None;
        }
        let mean_x = self.sum_x / self.n;
        let mean_y = self.sum_y / self.n;
        let den = self.sum_x2 - self.n * mean_x * mean_x;
        if den.abs() <= f64::EPSILON {
            return None;
        }
        let slope = (self.sum_xy - self.n * mean_x * mean_y) / den;
        let intercept = mean_y - slope * mean_x;
        Some((intercept, slope))
    }
}

#[derive(Debug, Clone)]
enum Phase {
    NoSignal,
    /// Collecting raw arrival times (seconds since `epoch`) for the initial fit.
    Collecting {
        epoch: Instant,
        samples: Vec<f64>,
    },
    Tracking {
        epoch: Instant,
        /// Model value (seconds since `epoch`) at tick index 0.
        t_ref: f64,
        /// Current period estimate, in seconds. Tracks with gain `BETA`, kept
        /// responsive since it's what future prompts use for send-time prediction.
        period: f64,
        /// A separate least-squares fit over every good tick since `epoch`, used
        /// only for the displayed BPM (ARCHITECTURE.md §5.2: "smoothed for
        /// display only"). Its variance shrinks as more ticks accumulate, unlike
        /// `period`'s fixed-gain tracking noise floor, so display accuracy keeps
        /// improving the longer a steady tempo runs.
        display_fit: RunningFit,
        /// Index of the next tick to be fed into the PLL.
        k: u64,
        outlier_streak: u32,
        /// Boxed because `JitterRms` embeds a 96-entry array: keeping it out of
        /// line stops this variant from dwarfing the others (`NoSignal`,
        /// `Collecting`), which would otherwise bloat every `Phase` value.
        jitter: Box<JitterRms>,
        locked: bool,
    },
}

/// Model of the incoming tick stream: `T(k) = t_ref + k * period` (ARCHITECTURE.md
/// §5.2). Fully unit-testable: every method takes an explicit `Instant`.
#[derive(Debug, Clone)]
pub struct Estimator {
    phase: Phase,
    last_tick_at: Option<Instant>,
}

impl Default for Estimator {
    fn default() -> Self {
        Self::new()
    }
}

impl Estimator {
    pub fn new() -> Self {
        Self {
            phase: Phase::NoSignal,
            last_tick_at: None,
        }
    }

    /// Resets to `NoSignal`, discarding all model state (used when switching
    /// input sources).
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Feeds one incoming `0xF8` clock tick at time `at`.
    pub fn on_tick(&mut self, at: Instant) {
        self.last_tick_at = Some(at);

        match &mut self.phase {
            Phase::NoSignal => {
                self.phase = Phase::Collecting {
                    epoch: at,
                    samples: vec![0.0],
                };
            }
            Phase::Collecting { epoch, samples } => {
                samples.push(at.duration_since(*epoch).as_secs_f64());
                if samples.len() >= INITIAL_FIT_SAMPLES {
                    let (t_ref, period) = fit_least_squares(samples);
                    let period = period.clamp(PERIOD_MIN_SECS, PERIOD_MAX_SECS);
                    let mut display_fit = RunningFit::new();
                    for (k, &y) in samples.iter().enumerate() {
                        display_fit.push(k as f64, y);
                    }
                    self.phase = Phase::Tracking {
                        epoch: *epoch,
                        t_ref,
                        period,
                        display_fit,
                        k: samples.len() as u64,
                        outlier_streak: 0,
                        jitter: Box::new(JitterRms::new()),
                        locked: false,
                    };
                }
            }
            Phase::Tracking {
                epoch,
                t_ref,
                period,
                display_fit,
                k,
                outlier_streak,
                jitter,
                locked,
            } => {
                let elapsed = at.duration_since(*epoch).as_secs_f64();
                let predicted = *t_ref + (*k as f64) * *period;
                let err = elapsed - predicted;

                if err.abs() > OUTLIER_FACTOR * *period {
                    *outlier_streak += 1;
                    if *outlier_streak >= MAX_OUTLIER_STREAK {
                        // Tempo jump or dropout: start re-acquiring from here.
                        self.phase = Phase::Collecting {
                            epoch: at,
                            samples: vec![0.0],
                        };
                        return;
                    }
                } else {
                    *outlier_streak = 0;
                    *t_ref += ALPHA * err;
                    *period = (*period + BETA * err).clamp(PERIOD_MIN_SECS, PERIOD_MAX_SECS);
                    display_fit.push(*k as f64, elapsed);
                    jitter.push(err * 1000.0);
                    *locked = jitter.rms_ms() < LOCK_JITTER_THRESHOLD_MS;
                }
                *k += 1;
            }
        }
    }

    /// Must be called periodically (independent of incoming ticks) so a dropout
    /// can be detected even while nothing arrives.
    pub fn check_timeout(&mut self, now: Instant) {
        let Some(last) = self.last_tick_at else {
            return;
        };
        let period = match &self.phase {
            Phase::Tracking { period, .. } => *period,
            _ => 0.0,
        };
        let timeout = Duration::from_secs_f64((3.0 * period).max(NO_SIGNAL_FLOOR_SECS));
        if now.saturating_duration_since(last) >= timeout {
            self.phase = Phase::NoSignal;
            self.last_tick_at = None;
        }
    }

    pub fn lock_state(&self) -> LockState {
        match &self.phase {
            Phase::NoSignal => LockState::NoSignal,
            Phase::Collecting { .. } => LockState::Acquiring,
            Phase::Tracking { locked, .. } => {
                if *locked {
                    LockState::Locked
                } else {
                    LockState::Acquiring
                }
            }
        }
    }

    pub fn bpm(&self) -> f64 {
        match &self.phase {
            Phase::Tracking {
                display_fit,
                period,
                ..
            } => {
                let display_period = display_fit.fit().map(|(_, slope)| slope).unwrap_or(*period);
                60.0 / (24.0 * display_period)
            }
            _ => 0.0,
        }
    }

    pub fn jitter_ms(&self) -> f64 {
        match &self.phase {
            Phase::Tracking { jitter, .. } => jitter.rms_ms(),
            _ => 0.0,
        }
    }

    /// The current tick-time model, for the scheduler (ARCHITECTURE.md §5.3).
    /// `Some` whenever a least-squares fit exists — i.e. during `Tracking`,
    /// regardless of whether jitter has settled enough to be `Locked` — and
    /// `None` otherwise (`NoSignal`, or still `Collecting` the first samples),
    /// which is exactly when the scheduler must stay silent.
    pub fn model(&self) -> Option<TickModel> {
        match &self.phase {
            Phase::Tracking {
                epoch,
                t_ref,
                period,
                k,
                ..
            } => Some(TickModel {
                epoch: *epoch,
                t_ref_secs: *t_ref,
                period_secs: *period,
                next_k: *k,
            }),
            _ => None,
        }
    }
}

/// A snapshot of the tick-time model `T(k) = t_ref + k * period` (relative to
/// `epoch`), exposed so the scheduler can predict future input tick times using
/// the *latest* model (ARCHITECTURE.md §5.3: "always use the latest model").
#[derive(Debug, Clone, Copy)]
pub struct TickModel {
    pub(crate) epoch: Instant,
    pub(crate) t_ref_secs: f64,
    pub(crate) period_secs: f64,
    /// The next input tick index the estimator expects; a reasonable starting
    /// point for an output that has no tick position of its own yet.
    pub next_k: u64,
}

impl TickModel {
    /// Predicted `Instant` at which input tick `k` will arrive.
    pub fn predicted_instant(&self, k: u64) -> Instant {
        offset_instant(self.epoch, self.t_ref_secs + k as f64 * self.period_secs)
    }

    pub fn period(&self) -> Duration {
        Duration::from_secs_f64(self.period_secs)
    }

    /// The input tick index whose predicted arrival time is closest to `at`.
    ///
    /// Used by the Measure panel (PROMPTS.md Prompt 7) to correlate an
    /// independently-measured MIDI event — e.g. `Clock Shifter Out` looped
    /// back into a second port — with "the same" input tick, since no index
    /// is carried in the byte stream itself. The residual between this tick's
    /// predicted arrival and `at` is the real-world measured lead.
    pub fn nearest_tick_index(&self, at: Instant) -> u64 {
        let elapsed = at.duration_since(self.epoch).as_secs_f64();
        let k = ((elapsed - self.t_ref_secs) / self.period_secs).round();
        if k <= 0.0 { 0 } else { k as u64 }
    }
}

/// Adds a possibly-negative offset (in seconds) to an `Instant`.
pub fn offset_instant(base: Instant, secs: f64) -> Instant {
    if secs >= 0.0 {
        base + Duration::from_secs_f64(secs)
    } else {
        base - Duration::from_secs_f64(-secs)
    }
}

/// Signed milliseconds `a - b` (positive if `a` is after `b`). `Instant`
/// subtraction alone can't express a negative result, and
/// `Instant::duration_since` silently saturates to zero rather than going
/// negative, which would corrupt a lead measurement that happens to come out
/// negative (arrived late).
pub fn signed_millis(a: Instant, b: Instant) -> f64 {
    if a >= b {
        a.duration_since(b).as_secs_f64() * 1000.0
    } else {
        -(b.duration_since(a).as_secs_f64() * 1000.0)
    }
}

/// Ordinary least-squares fit of `samples[k] ≈ t_ref + k * period`, returning
/// `(t_ref, period)`.
fn fit_least_squares(samples: &[f64]) -> (f64, f64) {
    let n = samples.len() as f64;
    let mean_x = (samples.len() as f64 - 1.0) / 2.0;
    let mean_y = samples.iter().sum::<f64>() / n;

    let mut num = 0.0;
    let mut den = 0.0;
    for (k, &y) in samples.iter().enumerate() {
        let dx = k as f64 - mean_x;
        num += dx * (y - mean_y);
        den += dx * dx;
    }

    let period = if den.abs() > f64::EPSILON {
        num / den
    } else {
        0.0
    };
    let t_ref = mean_y - period * mean_x;
    (t_ref, period)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic pseudo-random generator (splitmix64) so jitter in tests is
    /// reproducible without adding a `rand` dependency.
    struct SplitMix64(u64);

    impl SplitMix64 {
        fn new(seed: u64) -> Self {
            Self(seed)
        }

        fn next_u64(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
            z ^ (z >> 31)
        }

        /// Standard normal sample via Box-Muller.
        fn next_gaussian(&mut self) -> f64 {
            let u1 = (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
            let u2 = (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
            let u1 = u1.max(f64::MIN_POSITIVE);
            (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
        }
    }

    fn period_for_bpm(bpm: f64) -> Duration {
        Duration::from_secs_f64(60.0 / (bpm * 24.0))
    }

    /// Feeds `count` steady ticks at `bpm`, with Gaussian jitter (std-dev
    /// `jitter_ms`) applied to each tick's arrival time.
    fn feed_steady_ticks(
        estimator: &mut Estimator,
        clock_time: &mut Instant,
        bpm: f64,
        jitter_ms: f64,
        count: usize,
        rng: &mut SplitMix64,
    ) {
        let period = period_for_bpm(bpm);
        for _ in 0..count {
            *clock_time += period;
            let at = if jitter_ms > 0.0 {
                let jitter_secs = rng.next_gaussian() * jitter_ms / 1000.0;
                offset_instant(*clock_time, jitter_secs)
            } else {
                *clock_time
            };
            estimator.on_tick(at);
        }
    }

    #[test]
    fn steady_125_bpm_locks_and_is_accurate() {
        for jitter_ms in [0.0, 2.0] {
            let mut estimator = Estimator::new();
            let mut clock_time = Instant::now();
            let mut rng = SplitMix64::new(42);

            feed_steady_ticks(
                &mut estimator,
                &mut clock_time,
                125.0,
                jitter_ms,
                48,
                &mut rng,
            );
            assert_eq!(
                estimator.lock_state(),
                LockState::Locked,
                "should be locked within 48 ticks (jitter={jitter_ms}ms)"
            );

            // Let it settle further so PLL noise-averaging has time to work.
            feed_steady_ticks(
                &mut estimator,
                &mut clock_time,
                125.0,
                jitter_ms,
                300,
                &mut rng,
            );
            let bpm = estimator.bpm();
            assert!(bpm.is_finite());
            assert!(
                (bpm - 125.0).abs() < 0.05,
                "bpm={bpm} should be within 0.05 of 125 (jitter={jitter_ms}ms)"
            );
        }
    }

    #[test]
    fn tempo_step_relocks_without_nan_or_panic() {
        let mut estimator = Estimator::new();
        let mut clock_time = Instant::now();
        let mut rng = SplitMix64::new(7);

        feed_steady_ticks(&mut estimator, &mut clock_time, 125.0, 0.0, 48, &mut rng);
        assert_eq!(estimator.lock_state(), LockState::Locked);
        assert!(estimator.bpm().is_finite());

        // Tempo jump: 125 -> 130 BPM. The estimator isn't required to snap
        // instantly (a modest, sustained step doesn't necessarily trigger the
        // outlier/re-init path - ARCHITECTURE.md's PLL is designed to track
        // smoothly), just to converge close to the new tempo, and never panic
        // or produce NaN/infinite values while doing so.
        for _ in 0..3000 {
            feed_steady_ticks(&mut estimator, &mut clock_time, 130.0, 0.0, 1, &mut rng);
            let bpm = estimator.bpm();
            assert!(
                bpm.is_finite() && !bpm.is_nan(),
                "bpm went non-finite: {bpm}"
            );
        }

        let bpm = estimator.bpm();
        assert_eq!(estimator.lock_state(), LockState::Locked);
        assert!(
            (bpm - 130.0).abs() < 0.5,
            "bpm={bpm} should have re-locked near 130"
        );
    }

    #[test]
    fn dropout_goes_to_no_signal_and_recovers() {
        let mut estimator = Estimator::new();
        let mut clock_time = Instant::now();
        let mut rng = SplitMix64::new(99);

        feed_steady_ticks(&mut estimator, &mut clock_time, 125.0, 0.0, 48, &mut rng);
        assert_eq!(estimator.lock_state(), LockState::Locked);

        // 1s dropout: no ticks arrive, only the periodic timeout check runs.
        clock_time += Duration::from_secs(1);
        estimator.check_timeout(clock_time);
        assert_eq!(estimator.lock_state(), LockState::NoSignal);

        // Resuming ticks should recover: Acquiring, then Locked again.
        feed_steady_ticks(&mut estimator, &mut clock_time, 125.0, 0.0, 1, &mut rng);
        assert_eq!(estimator.lock_state(), LockState::Acquiring);

        feed_steady_ticks(&mut estimator, &mut clock_time, 125.0, 0.0, 48, &mut rng);
        assert_eq!(estimator.lock_state(), LockState::Locked);
    }

    #[test]
    fn single_outlier_tick_barely_moves_the_model() {
        let mut estimator = Estimator::new();
        let mut clock_time = Instant::now();
        let mut rng = SplitMix64::new(3);

        feed_steady_ticks(&mut estimator, &mut clock_time, 125.0, 0.0, 60, &mut rng);
        assert_eq!(estimator.lock_state(), LockState::Locked);
        let bpm_before = estimator.bpm();

        // One tick arrives 15ms late (period at 125 BPM is 20ms, so this exceeds
        // the 0.5*period outlier threshold and must be rejected). The true
        // schedule (`clock_time`) still only advances by one normal period: only
        // this single tick's *reported* arrival is perturbed, not the underlying
        // clock, so the next tick after it is back on the original schedule.
        clock_time += period_for_bpm(125.0);
        let outlier_at = clock_time + Duration::from_millis(15);
        estimator.on_tick(outlier_at);

        // A single outlier must not trigger re-acquisition.
        assert_eq!(estimator.lock_state(), LockState::Locked);

        feed_steady_ticks(&mut estimator, &mut clock_time, 125.0, 0.0, 10, &mut rng);
        let bpm_after = estimator.bpm();

        assert!(bpm_after.is_finite());
        assert!(
            (bpm_after - bpm_before).abs() < 0.05,
            "bpm moved from {bpm_before} to {bpm_after}, expected barely any movement"
        );
    }

    fn model_at(epoch: Instant, bpm: f64) -> TickModel {
        TickModel {
            epoch,
            t_ref_secs: 0.0,
            period_secs: 60.0 / (bpm * 24.0),
            next_k: 0,
        }
    }

    #[test]
    fn nearest_tick_index_finds_the_closest_predicted_tick() {
        let epoch = Instant::now();
        let model = model_at(epoch, 125.0); // period = 20ms

        assert_eq!(model.nearest_tick_index(epoch), 0);
        assert_eq!(
            model.nearest_tick_index(model.predicted_instant(7)),
            7,
            "an exact predicted instant must resolve to its own tick"
        );
        // A tick that arrived 6ms early (closer to k=10 than k=9 at a 20ms period).
        let near_ten = model.predicted_instant(10) - Duration::from_millis(6);
        assert_eq!(model.nearest_tick_index(near_ten), 10);
        // Before the epoch: clamps to 0 rather than underflowing.
        assert_eq!(model.nearest_tick_index(epoch - Duration::from_secs(1)), 0);
    }

    #[test]
    fn signed_millis_is_positive_when_a_is_after_b_and_negative_otherwise() {
        let base = Instant::now();
        let later = base + Duration::from_millis(150);
        assert!((signed_millis(later, base) - 150.0).abs() < 1e-6);
        assert!((signed_millis(base, later) - (-150.0)).abs() < 1e-6);
        assert_eq!(signed_millis(base, base), 0.0);
    }
}
