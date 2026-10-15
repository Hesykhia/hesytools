//! Synthetic input via `SendInput`.
//!
//! Two details matter for games, and both were wrong in the old tool:
//! - Keys are sent as hardware **scancodes** (`KEYEVENTF_SCANCODE`). Games that
//!   read raw input / DirectInput ignore virtual-key-only events.
//! - Clicks don't move the cursor unless asked to, and moves can be relative,
//!   which is what a captured in-game cursor (e.g. Minecraft) responds to.
//!
//! Every event carries [`SIGNATURE`] in `dwExtraInfo` so our own hooks can tell
//! injected events apart from the user's physical input.

use crate::keys::{Key, MouseButton};
use windows::Win32::Foundation::POINT;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
    SM_YVIRTUALSCREEN, XBUTTON1, XBUTTON2,
};

/// "HESY" — tag on every event we inject.
pub const SIGNATURE: usize = 0x4845_5359;

/// One wheel notch.
pub const WHEEL_NOTCH: i32 = 120;

pub fn is_ours(extra_info: usize) -> bool {
    extra_info == SIGNATURE
}

/// Send a batch of events atomically (nothing else can interleave).
/// Returns how many were accepted; fewer than `inputs.len()` usually means
/// UIPI blocked us because the target window runs elevated.
pub fn send(inputs: &[INPUT]) -> u32 {
    if inputs.is_empty() {
        return 0;
    }
    unsafe { SendInput(inputs, size_of::<INPUT>() as i32) }
}

pub fn key_event(key: Key, down: bool) -> INPUT {
    // MAPVK_VK_TO_VSC_EX puts 0xE0/0xE1 in the high byte for extended keys
    // (arrows, Insert/Delete/Home/End/PgUp/PgDn, right Ctrl/Alt, ...).
    let sc = unsafe { MapVirtualKeyW(key.0 as u32, MAPVK_VK_TO_VSC_EX) };
    let (vk, scan, mut flags) = if sc == 0 {
        // No scancode (some media/OEM keys): fall back to a VK event.
        (VIRTUAL_KEY(key.0), 0u16, KEYBD_EVENT_FLAGS(0))
    } else {
        let mut f = KEYEVENTF_SCANCODE;
        if sc & 0xFF00 != 0 {
            f |= KEYEVENTF_EXTENDEDKEY;
        }
        (VIRTUAL_KEY(0), (sc & 0xFF) as u16, f)
    };
    if !down {
        flags |= KEYEVENTF_KEYUP;
    }
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: scan,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: SIGNATURE,
            },
        },
    }
}

/// One UTF-16 unit typed as a character, independent of keyboard layout.
pub fn unicode_event(unit: u16, down: bool) -> INPUT {
    let mut flags = KEYEVENTF_UNICODE;
    if !down {
        flags |= KEYEVENTF_KEYUP;
    }
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(0),
                wScan: unit,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: SIGNATURE,
            },
        },
    }
}

fn mouse_event(dx: i32, dy: i32, data: u32, flags: MOUSE_EVENT_FLAGS) -> INPUT {
    INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx,
                dy,
                mouseData: data,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: SIGNATURE,
            },
        },
    }
}

pub fn button_event(button: MouseButton, down: bool) -> INPUT {
    let (flags, data) = match (button, down) {
        (MouseButton::Left, true) => (MOUSEEVENTF_LEFTDOWN, 0),
        (MouseButton::Left, false) => (MOUSEEVENTF_LEFTUP, 0),
        (MouseButton::Right, true) => (MOUSEEVENTF_RIGHTDOWN, 0),
        (MouseButton::Right, false) => (MOUSEEVENTF_RIGHTUP, 0),
        (MouseButton::Middle, true) => (MOUSEEVENTF_MIDDLEDOWN, 0),
        (MouseButton::Middle, false) => (MOUSEEVENTF_MIDDLEUP, 0),
        (MouseButton::X1, true) => (MOUSEEVENTF_XDOWN, XBUTTON1 as u32),
        (MouseButton::X1, false) => (MOUSEEVENTF_XUP, XBUTTON1 as u32),
        (MouseButton::X2, true) => (MOUSEEVENTF_XDOWN, XBUTTON2 as u32),
        (MouseButton::X2, false) => (MOUSEEVENTF_XUP, XBUTTON2 as u32),
    };
    mouse_event(0, 0, data, flags)
}

/// Absolute move to physical-pixel screen coordinates (multi-monitor aware).
pub fn move_abs_event(x: i32, y: i32) -> INPUT {
    let (vx, vy, vw, vh) = virtual_screen();
    // SendInput maps normalized n -> pixel floor(n * w / 65536), so round up
    // to land exactly on the requested pixel.
    let nx = (((x - vx) as i64 * 65536 + vw as i64 - 1) / vw.max(1) as i64) as i32;
    let ny = (((y - vy) as i64 * 65536 + vh as i64 - 1) / vh.max(1) as i64) as i32;
    mouse_event(
        nx,
        ny,
        0,
        MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK,
    )
}

/// Relative move in raw mouse counts (what games with a locked cursor read).
pub fn move_rel_event(dx: i32, dy: i32) -> INPUT {
    mouse_event(dx, dy, 0, MOUSEEVENTF_MOVE)
}

/// Wheel scroll. Positive = up/right. `amount` is in wheel units (120 per notch).
pub fn wheel_event(amount: i32, horizontal: bool) -> INPUT {
    let flags = if horizontal {
        MOUSEEVENTF_HWHEEL
    } else {
        MOUSEEVENTF_WHEEL
    };
    mouse_event(0, 0, amount as u32, flags)
}

pub fn key(key: Key, down: bool) -> u32 {
    send(&[key_event(key, down)])
}

pub fn tap_key(k: Key) -> u32 {
    send(&[key_event(k, true), key_event(k, false)])
}

pub fn button(b: MouseButton, down: bool) -> u32 {
    send(&[button_event(b, down)])
}

pub fn move_abs(x: i32, y: i32) -> u32 {
    send(&[move_abs_event(x, y)])
}

pub fn move_rel(dx: i32, dy: i32) -> u32 {
    send(&[move_rel_event(dx, dy)])
}

pub fn scroll(amount: i32, horizontal: bool) -> u32 {
    send(&[wheel_event(amount, horizontal)])
}

pub fn cursor_pos() -> (i32, i32) {
    let mut p = POINT::default();
    let _ = unsafe { GetCursorPos(&mut p) };
    (p.x, p.y)
}

/// Top-level window under a screen point, as a raw handle value.
pub fn root_window_at(x: i32, y: i32) -> isize {
    use windows::Win32::UI::WindowsAndMessaging::{GA_ROOT, GetAncestor, WindowFromPoint};
    unsafe { GetAncestor(WindowFromPoint(POINT { x, y }), GA_ROOT).0 as isize }
}

/// The window that currently has keyboard focus, as a raw handle value.
pub fn foreground_window() -> isize {
    unsafe { windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow().0 as isize }
}

/// Bounding box of all monitors: (left, top, width, height) in physical pixels.
pub fn virtual_screen() -> (i32, i32, i32, i32) {
    unsafe {
        (
            GetSystemMetrics(SM_XVIRTUALSCREEN),
            GetSystemMetrics(SM_YVIRTUALSCREEN),
            GetSystemMetrics(SM_CXVIRTUALSCREEN),
            GetSystemMetrics(SM_CYVIRTUALSCREEN),
        )
    }
}

/// Make this process DPI-aware so coordinates are physical pixels.
/// (The old tool skipped this, so positions were wrong on scaled displays.)
/// Tauri's manifest already does this for the app; examples/tests call it.
pub fn ensure_dpi_aware() {
    use windows::Win32::UI::HiDpi::{
        DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
    };
    let _ = unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
}
