//! Finding app windows, for Screen Watch's "only look at this app".
//!
//! A window is remembered by its program and title rather than its handle,
//! so a watch keeps working after the app is closed and opened again.

use serde::{Deserialize, Serialize};
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, RECT};
use windows::Win32::Graphics::Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute};
use windows::Win32::System::Threading::{
    GetCurrentProcessId, OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::{BOOL, PWSTR};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct WindowTarget {
    /// Program file name, e.g. "RobloxPlayerBeta.exe".
    pub exe: String,
    /// Window title when it was picked; preferred when the program has
    /// several windows, but not required (titles often change).
    pub title: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowInfo {
    pub exe: String,
    pub title: String,
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    /// Raw handle, only valid until the window closes.
    pub handle: isize,
}

/// Normal app windows on screen right now (largest first), excluding ours.
pub fn list() -> Vec<WindowInfo> {
    let mut out: Vec<WindowInfo> = Vec::new();
    unsafe {
        let _ = EnumWindows(Some(collect), LPARAM(&mut out as *mut _ as isize));
    }
    out.sort_by_key(|w| std::cmp::Reverse(w.w as u64 * w.h as u64));
    out
}

unsafe extern "system" fn collect(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let out = unsafe { &mut *(lparam.0 as *mut Vec<WindowInfo>) };
    if let Some(info) = describe(hwnd) {
        out.push(info);
    }
    true.into()
}

fn describe(hwnd: HWND) -> Option<WindowInfo> {
    unsafe {
        if !IsWindowVisible(hwnd).as_bool() || GetWindow(hwnd, GW_OWNER).is_ok_and(|o| !o.is_invalid()) {
            return None;
        }
        let ex = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
        if ex & WS_EX_TOOLWINDOW.0 != 0 {
            return None;
        }
        // Hidden UWP/virtual-desktop windows report visible but are cloaked.
        let mut cloaked = 0u32;
        let _ = DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED, &mut cloaked as *mut _ as *mut _, 4);
        if cloaked != 0 {
            return None;
        }
        let mut buf = [0u16; 512];
        let n = GetWindowTextW(hwnd, &mut buf);
        let title = String::from_utf16_lossy(&buf[..n.max(0) as usize]);
        if title.trim().is_empty() {
            return None;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == GetCurrentProcessId() {
            return None;
        }
        let exe = exe_name(pid)?;
        let (x, y, w, h) = rect(hwnd.0 as isize)?;
        Some(WindowInfo { exe, title, x, y, w, h, handle: hwnd.0 as isize })
    }
}

pub fn exe_name(pid: u32) -> Option<String> {
    unsafe {
        let proc = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(proc, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len).is_ok();
        let _ = CloseHandle(proc);
        if !ok {
            return None;
        }
        let path = String::from_utf16_lossy(&buf[..len as usize]);
        path.rsplit(['\\', '/']).next().map(str::to_string)
    }
}

/// Screen rectangle of a window (physical pixels), if it still exists.
pub fn rect(handle: isize) -> Option<(i32, i32, u32, u32)> {
    unsafe {
        let hwnd = HWND(handle as *mut _);
        if !IsWindow(Some(hwnd)).as_bool() {
            return None;
        }
        let mut r = RECT::default();
        GetWindowRect(hwnd, &mut r).ok()?;
        Some((r.left, r.top, (r.right - r.left).max(0) as u32, (r.bottom - r.top).max(0) as u32))
    }
}

pub fn is_minimized(handle: isize) -> bool {
    unsafe { IsIconic(HWND(handle as *mut _)).as_bool() }
}

/// The window for `t`: same program, preferring the same title, else the
/// program's biggest window.
pub fn find(t: &WindowTarget) -> Option<isize> {
    let all = list();
    let mine = all.iter().filter(|w| w.exe.eq_ignore_ascii_case(&t.exe));
    mine.clone().find(|w| w.title == t.title).or_else(|| mine.clone().next()).map(|w| w.handle)
}
