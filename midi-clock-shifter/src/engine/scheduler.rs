//! Per-output send times, slewing, monotonic tick emission (ARCHITECTURE.md §5.3).
//!
//! Pure logic, no I/O and no real waiting: [`OutputScheduler::peek_next`] just
//! computes when a tick *should* be sent given the current model; the caller
//! (the engine thread) decides when "now" has reached that time and actually
//! sends, then reports back via [`OutputScheduler::commit`]. This split is what
//! makes the scheduling math fully unit-testable with simulated time.

use std::time::Instant;

use super::estimator::{TickModel, offset_instant};

/// Max change to an output's effective (offset + trim) shift per tick, so a
/// config change is never seen as a tempo jump (ARCHITECTURE.md §5.3: "Slewing").
const MAX_SLEW_PER_TICK_SECS: f64 = 0.002;

/// Scheduling state for a single output.
#[derive(Debug, Clone)]
pub struct OutputScheduler {
    /// Next output tick index to send; `None` until (re)initialised from a model.
    next_k: Option<u64>,
    /// When the last tick was actually sent, for the catch-up spacing rule.
    last_sent_at: Option<Instant>,
    /// Currently-applied (offset + trim), slewed gradually toward the target.
    effective_shift_secs: f64,
    shift_initialized: bool,
    /// Input tick index a pending output `0xFA` must be sent immediately
    /// before (ARCHITECTURE.md §5.4). Not touched by [`Self::reset`]: a start
    /// pending across a signal dropout should still fire once ticks resume.
    pending_start: Option<u64>,
    /// The last tick index actually committed, for the UI's output position
    /// display (ARCHITECTURE.md §9: "beat cells ... of the output position").
    last_committed_k: Option<u64>,
}

impl Default for OutputScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputScheduler {
    pub fn new() -> Self {
        Self {
            next_k: None,
            last_sent_at: None,
            effective_shift_secs: 0.0,
            shift_initialized: false,
            pending_start: None,
            last_committed_k: None,
        }
    }

    /// Forgets the current tick position (but not the slewed shift value), so
    /// the output starts fresh at the model's current tick once one reappears.
    /// Called when the estimator has no model (`NoSignal`, or still collecting
    /// the first samples) — ARCHITECTURE.md §5.3: "stop sending ticks".
    pub fn reset(&mut self) {
        self.next_k = None;
        self.last_sent_at = None;
    }

    /// This output's next not-yet-sent tick index — the earliest index a
    /// pending resync boundary may still land on. A lead means ticks up to
    /// and including an already-past downbeat may already have gone out
    /// (ARCHITECTURE.md §5.4: "the downbeat itself is already past for all
    /// outputs with a lead").
    pub fn next_unsent_k(&self, model: &TickModel) -> u64 {
        self.next_k.unwrap_or(model.next_k)
    }

    /// Arms (or replaces) a pending start: an `0xFA` will be sent immediately
    /// before output tick `target_k` (ARCHITECTURE.md §5.4: "a newer start
    /// replaces a pending one").
    pub fn arm_start(&mut self, target_k: u64) {
        self.pending_start = Some(target_k);
    }

    /// Cancels a pending start without sending it (ARCHITECTURE.md §5.4: an
    /// incoming `0xFC` "cancels pending starts").
    pub fn cancel_start(&mut self) {
        self.pending_start = None;
    }

    pub fn has_pending_start(&self) -> bool {
        self.pending_start.is_some()
    }

    /// If `k` is the pending start's target, consumes it (so it fires exactly
    /// once) and reports that an `0xFA` must be sent immediately before this
    /// tick's `0xF8`.
    pub fn take_due_start(&mut self, k: u64) -> bool {
        if self.pending_start == Some(k) {
            self.pending_start = None;
            true
        } else {
            false
        }
    }

    /// The last tick index actually sent, for the UI's output position display.
    pub fn last_committed_k(&self) -> Option<u64> {
        self.last_committed_k
    }

    /// Computes `(k, send_at)` for the next pending tick, given the *latest*
    /// model and the target (offset + trim) shift, in seconds. Pure: calling
    /// this repeatedly without an intervening [`Self::commit`] always returns
    /// the same answer, so the caller can use it to decide how long to wait
    /// without accidentally advancing the slew more than once per tick.
    pub fn peek_next(&mut self, model: &TickModel, target_shift_secs: f64) -> (u64, Instant) {
        let k = *self.next_k.get_or_insert(model.next_k);
        let shift = if self.shift_initialized {
            self.effective_shift_secs
        } else {
            target_shift_secs
        };

        let mut send_at = offset_instant(model.predicted_instant(k), -shift);

        // Catch-up: never send two ticks closer together than half a period.
        if let Some(last) = self.last_sent_at {
            let min_next = last + model.period().mul_f64(0.5);
            if send_at < min_next {
                send_at = min_next;
            }
        }

        (k, send_at)
    }

    /// Records that tick `k` was actually sent at `sent_at`; advances the tick
    /// index and slews the effective shift by at most `MAX_SLEW_PER_TICK_SECS`
    /// toward `target_shift_secs`. Call this exactly once per tick actually sent.
    pub fn commit(&mut self, k: u64, sent_at: Instant, target_shift_secs: f64) {
        if self.shift_initialized {
            let delta = (target_shift_secs - self.effective_shift_secs)
                .clamp(-MAX_SLEW_PER_TICK_SECS, MAX_SLEW_PER_TICK_SECS);
            self.effective_shift_secs += delta;
        } else {
            self.effective_shift_secs = target_shift_secs;
            self.shift_initialized = true;
        }
        self.next_k = Some(k + 1);
        self.last_sent_at = Some(sent_at);
        self.last_committed_k = Some(k);
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    fn model_at(epoch: Instant, bpm: f64, next_k: u64) -> TickModel {
        TickModel {
            epoch,
            t_ref_secs: 0.0,
            period_secs: 60.0 / (bpm * 24.0),
            next_k,
        }
    }

    fn abs_diff(a: Instant, b: Instant) -> Duration {
        if a >= b { a - b } else { b - a }
    }

    /// Runs `count` ticks end-to-end (peek + commit as if sent exactly on
    /// schedule) and returns the `(k, send_at)` pairs.
    fn run_ticks(
        sched: &mut OutputScheduler,
        model: &TickModel,
        shift_secs: f64,
        count: usize,
    ) -> Vec<(u64, Instant)> {
        (0..count)
            .map(|_| {
                let (k, send_at) = sched.peek_next(model, shift_secs);
                sched.commit(k, send_at, shift_secs);
                (k, send_at)
            })
            .collect()
    }

    #[test]
    fn offset_200ms_sends_each_tick_that_much_early() {
        let epoch = Instant::now();
        let model = model_at(epoch, 125.0, 0);
        let mut sched = OutputScheduler::new();

        for (k, send_at) in run_ticks(&mut sched, &model, 0.2, 50) {
            let predicted = model.predicted_instant(k);
            let expected = predicted - Duration::from_secs_f64(0.2);
            assert!(
                abs_diff(send_at, expected) < Duration::from_millis(1),
                "tick {k}: send_at={send_at:?} expected~{expected:?}"
            );
        }
    }

    #[test]
    fn negative_offset_sends_each_tick_that_much_late() {
        let epoch = Instant::now();
        let model = model_at(epoch, 125.0, 0);
        let mut sched = OutputScheduler::new();

        for (k, send_at) in run_ticks(&mut sched, &model, -0.15, 30) {
            let predicted = model.predicted_instant(k);
            let expected = predicted + Duration::from_secs_f64(0.15);
            assert!(
                abs_diff(send_at, expected) < Duration::from_millis(1),
                "tick {k}: send_at={send_at:?} expected~{expected:?}"
            );
        }
    }

    #[test]
    fn offset_step_up_is_slewed_gradually_without_bunching_ticks() {
        let epoch = Instant::now();
        let model = model_at(epoch, 125.0, 0);
        let period = model.period();
        let mut sched = OutputScheduler::new();

        // Settle at 200ms first.
        let settled = run_ticks(&mut sched, &model, 0.2, 10);
        let mut prev_send_at = settled.last().unwrap().1;

        // Step to 300ms: must not jump immediately, and consecutive ticks must
        // never be closer than half a period apart.
        let mut saw_partial_progress = false;
        for _ in 0..200 {
            let (k, send_at) = sched.peek_next(&model, 0.3);
            assert!(
                send_at >= prev_send_at + period.mul_f64(0.5),
                "ticks too close: prev={prev_send_at:?} next={send_at:?} half_period={:?}",
                period.mul_f64(0.5)
            );
            sched.commit(k, send_at, 0.3);

            let predicted = model.predicted_instant(k);
            let full_target = predicted - Duration::from_secs_f64(0.3);
            if send_at != full_target {
                saw_partial_progress = true;
            }
            prev_send_at = send_at;
        }
        assert!(
            saw_partial_progress,
            "offset should reach 300ms gradually, not in one jump"
        );

        // After enough ticks, we should be at (or essentially at) the new target.
        let (k, send_at) = sched.peek_next(&model, 0.3);
        let predicted = model.predicted_instant(k);
        let expected = predicted - Duration::from_secs_f64(0.3);
        assert!(abs_diff(send_at, expected) < Duration::from_millis(1));
    }

    #[test]
    fn offset_step_down_never_repeats_or_reverses_a_tick() {
        let epoch = Instant::now();
        let model = model_at(epoch, 125.0, 0);
        let mut sched = OutputScheduler::new();

        // Settle at 300ms first.
        run_ticks(&mut sched, &model, 0.3, 10);

        // Step down to 200ms and keep ticking; k must strictly increase and
        // send_at must strictly increase (no negative intervals).
        let mut last_k: Option<u64> = None;
        let mut last_send_at: Option<Instant> = None;
        for _ in 0..200 {
            let (k, send_at) = sched.peek_next(&model, 0.2);
            if let Some(lk) = last_k {
                assert!(
                    k == lk + 1,
                    "tick index must increase by exactly 1, got {lk} -> {k}"
                );
            }
            if let Some(last) = last_send_at {
                assert!(
                    send_at > last,
                    "send_at must strictly increase: {last:?} -> {send_at:?}"
                );
            }
            sched.commit(k, send_at, 0.2);
            last_k = Some(k);
            last_send_at = Some(send_at);
        }
    }

    #[test]
    fn two_outputs_with_different_trims_are_independent() {
        let epoch = Instant::now();
        let model = model_at(epoch, 125.0, 0);
        let mut sched_a = OutputScheduler::new();
        let mut sched_b = OutputScheduler::new();

        let offset = 0.2;
        let trim_a = 0.01; // +10ms
        let trim_b = -0.02; // -20ms

        for _ in 0..30 {
            let (ka, at_a) = sched_a.peek_next(&model, offset + trim_a);
            let (kb, at_b) = sched_b.peek_next(&model, offset + trim_b);
            assert_eq!(
                ka, kb,
                "both outputs should track the same input tick index"
            );

            let predicted = model.predicted_instant(ka);
            let expected_a = predicted - Duration::from_secs_f64(offset + trim_a);
            let expected_b = predicted - Duration::from_secs_f64(offset + trim_b);
            assert!(abs_diff(at_a, expected_a) < Duration::from_millis(1));
            assert!(abs_diff(at_b, expected_b) < Duration::from_millis(1));
            // Output B has a more negative trim (less lead), so it must fire later.
            assert!(at_b > at_a);

            sched_a.commit(ka, at_a, offset + trim_a);
            sched_b.commit(kb, at_b, offset + trim_b);
        }
    }
}
