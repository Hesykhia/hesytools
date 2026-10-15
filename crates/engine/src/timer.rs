//! Precise, cancellable waits.
//!
//! `std::thread::sleep` on Windows has ~1-15ms granularity, which is why the
//! old tool fell back to busy loops that pinned a CPU core. A high-resolution
//! waitable timer (Windows 10 1803+) wakes within a fraction of a millisecond
//! while the thread is genuinely asleep. Waits also watch a cancel event, so a
//! 10-second interval still stops instantly when the user hits the hotkey.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
use windows::Win32::System::Threading::{
    CREATE_WAITABLE_TIMER_HIGH_RESOLUTION, CreateEventW, CreateWaitableTimerExW, INFINITE,
    ResetEvent, SetEvent, SetWaitableTimer, TIMER_ALL_ACCESS, WaitForMultipleObjects,
};
use windows::core::PCWSTR;

/// Below this, spin instead of arming the timer; its wake-up latency would
/// otherwise dominate. Costs at most this much CPU per wait.
const SPIN_BELOW: Duration = Duration::from_micros(250);

struct OwnedHandle(HANDLE);
// Kernel handles are process-wide and safe to use from any thread.
unsafe impl Send for OwnedHandle {}
unsafe impl Sync for OwnedHandle {}
impl Drop for OwnedHandle {
    fn drop(&mut self) {
        let _ = unsafe { CloseHandle(self.0) };
    }
}

pub struct Waiter {
    timer: OwnedHandle,
    cancel: OwnedHandle,
    cancelled: AtomicBool,
}

impl Waiter {
    pub fn new() -> windows::core::Result<Self> {
        unsafe {
            let timer = CreateWaitableTimerExW(
                None,
                PCWSTR::null(),
                CREATE_WAITABLE_TIMER_HIGH_RESOLUTION,
                TIMER_ALL_ACCESS.0,
            )
            // Pre-1803 Windows: fall back to a normal-resolution timer.
            .or_else(|_| CreateWaitableTimerExW(None, PCWSTR::null(), 0, TIMER_ALL_ACCESS.0))?;
            let timer = OwnedHandle(timer);
            let cancel = OwnedHandle(CreateEventW(None, true, false, PCWSTR::null())?);
            Ok(Self {
                timer,
                cancel,
                cancelled: AtomicBool::new(false),
            })
        }
    }

    /// Sleep until `deadline`. Returns `false` if cancelled first.
    pub fn wait_until(&self, deadline: Instant) -> bool {
        loop {
            if self.is_cancelled() {
                return false;
            }
            let now = Instant::now();
            if now >= deadline {
                return true;
            }
            let remaining = deadline - now;
            if remaining < SPIN_BELOW {
                while Instant::now() < deadline {
                    if self.is_cancelled() {
                        return false;
                    }
                    std::hint::spin_loop();
                }
                return true;
            }
            // Wake slightly early and let the spin finish the job.
            let sleep = remaining - SPIN_BELOW / 2;
            // Negative due time = relative, in 100ns units.
            let due = -((sleep.as_nanos() / 100).max(1) as i64);
            unsafe {
                if SetWaitableTimer(self.timer.0, &due, 0, None, None, false).is_err() {
                    std::thread::sleep(sleep);
                    continue;
                }
                let r = WaitForMultipleObjects(&[self.cancel.0, self.timer.0], false, INFINITE);
                if r == WAIT_OBJECT_0 {
                    return false;
                }
            }
        }
    }

    pub fn wait(&self, d: Duration) -> bool {
        self.wait_until(Instant::now() + d)
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        let _ = unsafe { SetEvent(self.cancel.0) };
    }

    pub fn reset(&self) {
        self.cancelled.store(false, Ordering::SeqCst);
        let _ = unsafe { ResetEvent(self.cancel.0) };
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn waits_roughly_the_right_time() {
        let w = Waiter::new().unwrap();
        for ms in [1u64, 5, 20] {
            let t = Instant::now();
            assert!(w.wait(Duration::from_millis(ms)));
            let took = t.elapsed();
            assert!(took >= Duration::from_millis(ms), "{ms}ms took {took:?}");
            assert!(took < Duration::from_millis(ms + 5), "{ms}ms took {took:?}");
        }
    }

    #[test]
    fn cancel_interrupts_long_wait() {
        let w = Arc::new(Waiter::new().unwrap());
        let w2 = w.clone();
        let t = Instant::now();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(30));
            w2.cancel();
        });
        assert!(!w.wait(Duration::from_secs(10)));
        assert!(t.elapsed() < Duration::from_secs(1));
    }
}
