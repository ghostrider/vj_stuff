//! Start/stop/resync boundary math (ARCHITECTURE.md §5.4).
//!
//! Pure logic, no I/O: [`resync_target`] computes which input tick index a
//! pending output `0xFA` should attach to, given the current model, one
//! output's shift and its next not-yet-sent tick. The engine thread
//! (`engine/mod.rs`) owns the actual per-output state (armed via
//! [`super::scheduler::OutputScheduler::arm_start`]) and turns a due target
//! into a real send.

use std::time::Instant;

use super::estimator::{TickModel, offset_instant};

/// MIDI clock ticks per 4/4 bar at 24 PPQN.
pub const TICKS_PER_BAR: u64 = 96;

/// `Q` in ARCHITECTURE.md §5.4: ticks per resync quantum (1, 2 or 4 bars).
pub fn quantum_ticks(quantum_bars: u32) -> u64 {
    TICKS_PER_BAR * (quantum_bars.max(1) as u64)
}

/// Whether output ticks currently flow to forward-transport-enabled outputs.
/// An incoming `0xFC` stops them until the next `0xFA` (ARCHITECTURE.md §5.4);
/// outputs with `forward_transport = false` ignore this entirely (they "only
/// receive 0xF8").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RunState {
    #[default]
    Running,
    Stopped,
}

/// The smallest input tick index `k_d + n*Q` (`n >= 0`) that this output has
/// not already sent (`>= earliest_unsent`) and whose scheduled send time is
/// still in the future — the boundary a pending output `0xFA` attaches to
/// (ARCHITECTURE.md §5.4: "n is the smallest value for which `send_time` is
/// still in the future"; a lead means small `n` are often already past, so
/// zero/negative-offset outputs naturally land on `n = 0`).
pub fn resync_target(
    model: &TickModel,
    shift_secs: f64,
    k_d: u64,
    quantum_ticks: u64,
    earliest_unsent: u64,
    now: Instant,
) -> u64 {
    let mut n: u64 = 0;
    loop {
        let candidate = k_d + n * quantum_ticks;
        if candidate >= earliest_unsent {
            let send_at = offset_instant(model.predicted_instant(candidate), -shift_secs);
            if send_at > now {
                return candidate;
            }
        }
        n += 1;
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::engine::scheduler::OutputScheduler;

    fn model_at(epoch: Instant, bpm: f64, next_k: u64) -> TickModel {
        TickModel {
            epoch,
            t_ref_secs: 0.0,
            period_secs: 60.0 / (bpm * 24.0),
            next_k,
        }
    }

    /// ARCHITECTURE.md §5.4 acceptance example: with a lead offset, the
    /// downbeat itself is already past, so the pending `0xFA` lands on
    /// `k_d + Q`, sent the usual `offset` early.
    #[test]
    fn lead_offset_resyncs_before_the_tick_at_downbeat_plus_quantum() {
        let epoch = Instant::now();
        let bpm = 125.0;
        let model = model_at(epoch, bpm, 10);
        let k_d = 10;
        let q = quantum_ticks(1);
        let shift = 0.2; // 200ms lead, more than one tick period away.

        // Processing right before tick k_d arrives.
        let now = model.predicted_instant(k_d) - Duration::from_micros(1);

        let target = resync_target(&model, shift, k_d, q, k_d, now);
        assert_eq!(target, k_d + q, "should land on the next quantum boundary");

        let expected_send_at = model.predicted_instant(target) - Duration::from_secs_f64(shift);
        let send_at = offset_instant(model.predicted_instant(target), -shift);
        assert_eq!(send_at, expected_send_at);
        assert!(send_at > now, "must still be scheduled in the future");
    }

    /// Output tick indices must stay strictly monotonic across a start: the
    /// pending `0xFA` only ever prefixes a tick, it never skips or repeats one
    /// (ARCHITECTURE.md §5.3/§5.4).
    #[test]
    fn tick_count_stays_consistent_across_a_pending_start() {
        let epoch = Instant::now();
        let model = model_at(epoch, 125.0, 0);
        let mut sched = OutputScheduler::new();

        sched.arm_start(20);

        let mut last_k: Option<u64> = None;
        let mut fired_at = None;
        for _ in 0..40 {
            let (k, send_at) = sched.peek_next(&model, 0.0);
            if let Some(lk) = last_k {
                assert_eq!(k, lk + 1, "tick index must not gap or repeat");
            }
            if sched.take_due_start(k) {
                fired_at = Some(k);
            }
            sched.commit(k, send_at, 0.0);
            last_k = Some(k);
        }

        assert_eq!(
            fired_at,
            Some(20),
            "the pending start must fire exactly once, at tick 20"
        );
    }

    #[test]
    fn stop_cancels_a_pending_start() {
        let epoch = Instant::now();
        let model = model_at(epoch, 125.0, 0);
        let mut sched = OutputScheduler::new();

        sched.arm_start(5);
        assert!(sched.has_pending_start());
        sched.cancel_start();
        assert!(!sched.has_pending_start());

        for _ in 0..10 {
            let (k, send_at) = sched.peek_next(&model, 0.0);
            assert!(
                !sched.take_due_start(k),
                "a cancelled start must never fire"
            );
            sched.commit(k, send_at, 0.0);
        }
    }

    /// ARCHITECTURE.md §5.4 worked example: offset +4000ms with a 1-bar
    /// quantum at 60 BPM (one bar = 4000ms exactly) moves the resync to the
    /// second bar boundary (`n = 1`), because the first boundary's send time
    /// has already passed by the time it would be scheduled.
    #[test]
    fn large_lead_offset_pushes_resync_to_the_next_quantum_boundary() {
        let epoch = Instant::now();
        let model = model_at(epoch, 60.0, 0);
        let k_d = 0;
        let q = quantum_ticks(1);
        assert_eq!(model.predicted_instant(q) - epoch, Duration::from_secs(4));

        let now = model.predicted_instant(k_d) - Duration::from_micros(1);
        let target = resync_target(&model, 4.0, k_d, q, k_d, now);

        assert_eq!(target, k_d + q, "must move to the second bar (n = 1)");
    }

    /// ARCHITECTURE.md §5.4: a negative offset never needs to skip a
    /// boundary — the first one (`n = 0`) is always still in the future, and
    /// the `0xFA` is simply sent late.
    #[test]
    fn negative_offset_resyncs_at_the_first_boundary_and_sends_late() {
        let epoch = Instant::now();
        let model = model_at(epoch, 60.0, 0);
        let k_d = 0;
        let q = quantum_ticks(1);

        let now = model.predicted_instant(k_d) - Duration::from_micros(1);
        let target = resync_target(&model, -4.0, k_d, q, k_d, now);

        assert_eq!(target, k_d, "n = 0: the first boundary is used");
        let send_at = offset_instant(model.predicted_instant(target), 4.0);
        assert!(
            send_at > model.predicted_instant(k_d),
            "a negative offset sends the resync after the downbeat arrives"
        );
    }
}
