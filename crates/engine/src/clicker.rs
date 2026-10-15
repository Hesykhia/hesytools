//! The autoclicker: repeatedly clicks a mouse button or taps a key.
//!
//! Timing is deadline-based (`next += interval`), so a late wake-up is
//! absorbed by the following wait instead of drifting the rate.

use crate::input;
use crate::keys::{Key, MouseButton};
use crate::timer::Waiter;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ClickAction {
    Mouse { button: MouseButton },
    Key { key: Key },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ClickPosition {
    /// Wherever the cursor is. The cursor isn't touched.
    Current,
    /// Move to a fixed screen point before every click.
    Fixed { x: i32, y: i32 },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum StopAfter {
    Never,
    Clicks { count: u64 },
    Seconds { secs: f64 },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct ClickerConfig {
    pub action: ClickAction,
    /// Time between the start of one click and the next.
    pub interval_ms: f64,
    /// How long the button/key stays down. Games often need ~1 frame
    /// (10-20ms) to notice a key press at all.
    pub hold_ms: f64,
    /// Random ± offset applied to each interval.
    pub jitter_ms: f64,
    pub position: ClickPosition,
    pub stop_after: StopAfter,
    /// Double-click each time (two presses close together), like OP Auto
    /// Clicker's "click type: double".
    pub double: bool,
}

/// Gap between the two presses of a double click.
const DOUBLE_GAP: Duration = Duration::from_millis(30);

impl Default for ClickerConfig {
    fn default() -> Self {
        Self {
            action: ClickAction::Mouse { button: MouseButton::Left },
            interval_ms: 100.0,
            hold_ms: 10.0,
            jitter_ms: 0.0,
            position: ClickPosition::Current,
            stop_after: StopAfter::Never,
            double: false,
        }
    }
}

#[derive(Default)]
struct Shared {
    running: AtomicBool,
    clicks: AtomicU64,
    /// Events Windows refused to inject (e.g. blocked by an elevated window).
    blocked: AtomicU64,
}

struct Run {
    waiter: Arc<Waiter>,
    thread: JoinHandle<()>,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClickerStatus {
    pub running: bool,
    /// Clicks in the current/most recent run.
    pub clicks: u64,
    pub blocked: u64,
}

/// Why a run ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ExitReason {
    /// `stop()` was called.
    Stopped,
    /// The configured click count or duration was reached.
    Finished,
}

#[derive(Default)]
pub struct Clicker {
    run: Mutex<Option<Run>>,
    shared: Arc<Shared>,
}

impl Clicker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Start clicking on a background thread. `on_exit` runs on that thread
    /// when the run ends; it must not call back into `start`/`stop`.
    pub fn start(
        &self,
        cfg: ClickerConfig,
        on_exit: impl FnOnce(ExitReason) + Send + 'static,
    ) -> windows::core::Result<()> {
        self.stop();
        let waiter = Arc::new(Waiter::new()?);
        let shared = self.shared.clone();
        shared.clicks.store(0, Ordering::SeqCst);
        shared.blocked.store(0, Ordering::SeqCst);
        shared.running.store(true, Ordering::SeqCst);
        let w = waiter.clone();
        let thread = std::thread::Builder::new()
            .name("clicker".into())
            .spawn(move || {
                raise_thread_priority();
                click_loop(&cfg, &w, &shared);
                shared.running.store(false, Ordering::SeqCst);
                on_exit(if w.is_cancelled() { ExitReason::Stopped } else { ExitReason::Finished });
            })
            .expect("spawn clicker thread");
        *self.run.lock() = Some(Run { waiter, thread });
        Ok(())
    }

    pub fn stop(&self) {
        if let Some(run) = self.run.lock().take() {
            run.waiter.cancel();
            let _ = run.thread.join();
        }
    }

    pub fn is_running(&self) -> bool {
        self.shared.running.load(Ordering::SeqCst)
    }

    pub fn status(&self) -> ClickerStatus {
        ClickerStatus {
            running: self.is_running(),
            clicks: self.shared.clicks.load(Ordering::Relaxed),
            blocked: self.shared.blocked.load(Ordering::Relaxed),
        }
    }
}

impl Drop for Clicker {
    fn drop(&mut self) {
        self.stop();
    }
}

fn click_loop(cfg: &ClickerConfig, w: &Waiter, shared: &Shared) {
    let interval = cfg.interval_ms.max(0.5);
    // A hold longer than the interval would merge clicks into one long press.
    let hold = Duration::from_secs_f64(cfg.hold_ms.clamp(0.0, interval * 0.5) / 1000.0);
    let jitter = cfg.jitter_ms.clamp(0.0, interval * 0.9);
    let (down, up) = match &cfg.action {
        ClickAction::Mouse { button } => (input::button_event(*button, true), input::button_event(*button, false)),
        ClickAction::Key { key } => (input::key_event(*key, true), input::key_event(*key, false)),
    };
    let mut rng = XorShift::seeded();
    let start = Instant::now();
    let mut next = start;
    let mut count = 0u64;

    let send = |events: &[_]| {
        let sent = input::send(events);
        if (sent as usize) < events.len() {
            shared.blocked.fetch_add(1, Ordering::Relaxed);
        }
    };

    loop {
        match cfg.stop_after {
            StopAfter::Clicks { count: n } if count >= n => return,
            StopAfter::Seconds { secs } if start.elapsed().as_secs_f64() >= secs => return,
            _ => {}
        }
        if let ClickPosition::Fixed { x, y } = cfg.position {
            send(&[input::move_abs_event(x, y)]);
        }
        for press in 0..if cfg.double { 2 } else { 1 } {
            if press == 1 && !w.wait(DOUBLE_GAP) {
                return;
            }
            if hold.is_zero() {
                send(&[down, up]);
            } else {
                send(&[down]);
                let finished = w.wait(hold);
                // Always release, even when stopping mid-hold.
                send(&[up]);
                if !finished {
                    return;
                }
            }
        }
        count += 1;
        shared.clicks.store(count, Ordering::Relaxed);

        let offset = if jitter > 0.0 { rng.uniform(-jitter, jitter) } else { 0.0 };
        next += Duration::from_secs_f64((interval + offset) / 1000.0);
        let now = Instant::now();
        // After a long stall (system hiccup, debugger) don't fire a burst of
        // catch-up clicks; resume the normal rhythm from now.
        if now > next + Duration::from_millis(100) {
            next = now;
        }
        if !w.wait_until(next) {
            return;
        }
    }
}

fn raise_thread_priority() {
    use windows::Win32::System::Threading::{GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_HIGHEST};
    let _ = unsafe { SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_HIGHEST) };
}

/// Tiny PRNG for jitter; not worth a dependency.
struct XorShift(u64);

impl XorShift {
    fn seeded() -> Self {
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15);
        Self(t | 1)
    }

    fn next_f64(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }

    fn uniform(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.next_f64()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_json_shape() {
        let cfg = ClickerConfig {
            action: ClickAction::Key { key: Key::from_name("E").unwrap() },
            position: ClickPosition::Fixed { x: 10, y: 20 },
            stop_after: StopAfter::Clicks { count: 5 },
            ..Default::default()
        };
        let json = serde_json::to_string(&cfg).unwrap();
        assert!(json.contains(r#""action":{"kind":"key","key":"E"}"#), "{json}");
        assert_eq!(serde_json::from_str::<ClickerConfig>(&json).unwrap(), cfg);
        // Missing fields fall back to defaults, so old configs keep loading.
        let partial: ClickerConfig = serde_json::from_str(r#"{"intervalMs": 5}"#).unwrap();
        assert_eq!(partial.interval_ms, 5.0);
        assert_eq!(partial.action, ClickAction::Mouse { button: MouseButton::Left });
    }
}
