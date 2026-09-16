//! `TimeSource`: abstracts "now" so timing-sensitive logic (the estimator's
//! no-signal timeout, the simulator's pacing) can be driven deterministically in
//! tests instead of waiting on the real wall clock (ARCHITECTURE.md §5.2, §10).

#[cfg(test)]
use std::time::Duration;
use std::time::Instant;

/// A source of the current time. `RealTime` in production; a manually-advanced
/// clock in tests.
pub trait TimeSource: Send {
    fn now(&self) -> Instant;
}

/// Real wall-clock time (`Instant::now()`, backed by QPC on Windows).
#[derive(Debug, Default, Clone, Copy)]
pub struct RealTime;

impl TimeSource for RealTime {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

/// A clock that only moves forward when explicitly told to, for deterministic
/// tests. Never reflects real elapsed wall-clock time.
///
/// Only ever used from `#[cfg(test)]` code (hence gated the same way here):
/// production always uses [`RealTime`].
#[cfg(test)]
#[derive(Debug, Clone)]
pub struct SimClock {
    now: Instant,
}

#[cfg(test)]
impl SimClock {
    /// Starts the simulated clock at the real "now" (an arbitrary but valid
    /// `Instant` to build on; only relative advances matter afterwards).
    pub fn new() -> Self {
        Self {
            now: Instant::now(),
        }
    }

    pub fn advance(&mut self, by: Duration) {
        self.now += by;
    }
}

#[cfg(test)]
impl Default for SimClock {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
impl TimeSource for SimClock {
    fn now(&self) -> Instant {
        self.now
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sim_clock_only_advances_when_told() {
        let mut clock = SimClock::new();
        let t0 = clock.now();
        clock.advance(Duration::from_secs(1));
        assert_eq!(clock.now(), t0 + Duration::from_secs(1));
    }
}
