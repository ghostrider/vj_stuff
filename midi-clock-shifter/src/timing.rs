use windows::Win32::Media::{timeBeginPeriod, timeEndPeriod};
use windows::Win32::System::Threading::{
    GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_TIME_CRITICAL,
};

const TIMER_PERIOD_MS: u32 = 1;
const TIMERR_NOERROR: u32 = 0;

/// RAII guard around `timeBeginPeriod(1)` / `timeEndPeriod(1)`, raising the Windows
/// system timer resolution for the lifetime of the app so `spin_sleep` and the
/// scheduler thread get sub-10ms wakeups.
pub struct TimerResolutionGuard {
    active: bool,
}

impl TimerResolutionGuard {
    pub fn new() -> Self {
        let result = unsafe { timeBeginPeriod(TIMER_PERIOD_MS) };
        if result != TIMERR_NOERROR {
            log::warn!("timeBeginPeriod({TIMER_PERIOD_MS}) failed with code {result}");
            return Self { active: false };
        }
        Self { active: true }
    }
}

impl Default for TimerResolutionGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TimerResolutionGuard {
    fn drop(&mut self) {
        if self.active {
            let result = unsafe { timeEndPeriod(TIMER_PERIOD_MS) };
            if result != TIMERR_NOERROR {
                log::warn!("timeEndPeriod({TIMER_PERIOD_MS}) failed with code {result}");
            }
        }
    }
}

/// Raises the *current* thread to `THREAD_PRIORITY_TIME_CRITICAL`, for the
/// engine/scheduler thread (ARCHITECTURE.md §8). Logs and continues on failure
/// rather than panicking — a lower-priority scheduler thread degrades timing
/// accuracy but is still functional.
pub fn set_current_thread_time_critical() {
    // Safety: `GetCurrentThread()` returns a pseudo-handle to the calling
    // thread, valid for the duration of this call and requiring no cleanup.
    let handle = unsafe { GetCurrentThread() };
    if let Err(err) = unsafe { SetThreadPriority(handle, THREAD_PRIORITY_TIME_CRITICAL) } {
        log::warn!("SetThreadPriority(TIME_CRITICAL) failed: {err}");
    }
}
