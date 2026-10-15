//! "Only in this app": input may only go out while a chosen app is the one
//! in front, and mouse actions only where that app actually is under the
//! pointer. Alt-tabbing away, or the cursor drifting onto the taskbar, a
//! taskbar preview or another window, holds everything until it's back.

use crate::window::{self, WindowTarget};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::UI::WindowsAndMessaging::{GA_ROOT, GetAncestor, GetForegroundWindow, GetWindowThreadProcessId, WindowFromPoint};

pub struct AppLock {
    pub target: WindowTarget,
    /// Last foreground window and the pid if it was the target's program.
    /// Saves looking up the program name on every click.
    cache: Mutex<(isize, Option<u32>)>,
    /// Currently holding input back (for "waiting for <app>" in the UI).
    waiting: AtomicBool,
}

fn pid_of(hwnd: HWND) -> u32 {
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    pid
}

impl AppLock {
    pub fn new(target: WindowTarget) -> Self {
        Self { target, cache: Mutex::new((0, None)), waiting: AtomicBool::new(false) }
    }

    /// The target program's pid if it's the app in front right now.
    fn front_pid(&self) -> Option<u32> {
        let fg = unsafe { GetForegroundWindow() };
        let key = fg.0 as isize;
        let mut c = self.cache.lock();
        if c.0 != key {
            let pid = pid_of(fg);
            let ok = window::exe_name(pid).is_some_and(|e| e.eq_ignore_ascii_case(&self.target.exe));
            *c = (key, ok.then_some(pid));
        }
        c.1
    }

    /// May input go out now? `at` is where a mouse action would land (None
    /// for keys): the window there must belong to the app too.
    pub fn allows(&self, at: Option<(i32, i32)>) -> bool {
        let ok = self.front_pid().is_some_and(|pid| match at {
            None => true,
            Some((x, y)) => unsafe {
                let under = WindowFromPoint(POINT { x, y });
                !under.is_invalid() && pid_of(GetAncestor(under, GA_ROOT)) == pid
            },
        });
        self.waiting.store(!ok, Ordering::Relaxed);
        ok
    }

    pub fn is_waiting(&self) -> bool {
        self.waiting.load(Ordering::Relaxed)
    }

    /// Short name for messages: "RobloxPlayerBeta".
    pub fn app_name(&self) -> String {
        self.target.exe.trim_end_matches(".exe").trim_end_matches(".EXE").to_string()
    }
}
