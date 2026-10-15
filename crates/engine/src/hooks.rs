//! System-wide low-level keyboard/mouse hooks.
//!
//! One background thread owns both hooks and fans events out to subscribers
//! over channels. The hook callbacks only copy a struct and push it, because
//! Windows silently removes hooks whose callbacks are slow.
//!
//! Mouse-move events are only forwarded while someone has asked for them
//! (the recorder); hotkeys don't need them.

use crate::input;
use crate::keys::{Key, MouseButton};
use crossbeam_channel::{Receiver, Sender, unbounded};
use parking_lot::Mutex;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Instant;
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;

#[derive(Clone, Copy, Debug)]
pub enum InputKind {
    Key { key: Key, scan: u16, extended: bool, down: bool },
    Button { button: MouseButton, down: bool },
    Move,
    Wheel { amount: i32, horizontal: bool },
}

#[derive(Clone, Copy, Debug)]
pub struct InputEvent {
    pub kind: InputKind,
    /// Cursor position at the time of the event (mouse events only).
    pub x: i32,
    pub y: i32,
    /// True if hesytools injected this event itself.
    pub ours: bool,
    /// Injected by some *other* software.
    pub foreign_injected: bool,
    pub at: Instant,
}

struct Subscriber {
    tx: Sender<InputEvent>,
    moves: bool,
}

struct Hub {
    subs: Mutex<Vec<Subscriber>>,
    move_subs: AtomicUsize,
}

static HUB: OnceLock<Hub> = OnceLock::new();

fn hub() -> &'static Hub {
    HUB.get_or_init(|| {
        std::thread::Builder::new()
            .name("input-hooks".into())
            .spawn(hook_thread)
            .expect("spawn hook thread");
        Hub {
            subs: Mutex::new(Vec::new()),
            move_subs: AtomicUsize::new(0),
        }
    })
}

/// Receive every keyboard/mouse event system-wide until the receiver is dropped.
pub fn subscribe(want_moves: bool) -> Receiver<InputEvent> {
    let (tx, rx) = unbounded();
    let h = hub();
    if want_moves {
        h.move_subs.fetch_add(1, Ordering::SeqCst);
    }
    h.subs.lock().push(Subscriber { tx, moves: want_moves });
    rx
}

static PICKING: AtomicBool = AtomicBool::new(false);
static PICK_TX: Mutex<Option<Sender<Option<(i32, i32)>>>> = Mutex::new(None);
static SWALLOW_LEFT_UP: AtomicBool = AtomicBool::new(false);

/// Block until the user left-clicks anywhere and return that screen point.
/// The click itself is swallowed so it doesn't hit whatever is underneath.
/// Escape cancels (returns `None`).
pub fn pick_point() -> Option<(i32, i32)> {
    hub();
    let (tx, rx) = crossbeam_channel::bounded(1);
    *PICK_TX.lock() = Some(tx);
    PICKING.store(true, Ordering::SeqCst);
    rx.recv().ok().flatten()
}

pub fn cancel_pick() {
    finish_pick(None);
}

fn finish_pick(result: Option<(i32, i32)>) -> bool {
    PICKING.store(false, Ordering::SeqCst);
    match PICK_TX.lock().take() {
        Some(tx) => tx.send(result).is_ok(),
        None => false,
    }
}

fn publish(ev: InputEvent) {
    let Some(h) = HUB.get() else { return };
    let is_move = matches!(ev.kind, InputKind::Move);
    let mut subs = h.subs.lock();
    subs.retain(|s| {
        if is_move && !s.moves {
            return true;
        }
        let alive = s.tx.send(ev).is_ok();
        if !alive && s.moves {
            h.move_subs.fetch_sub(1, Ordering::SeqCst);
        }
        alive
    });
}

fn hook_thread() {
    unsafe {
        let hmod = GetModuleHandleW(None).ok().map(|m| m.into());
        let kb = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), hmod, 0);
        let ms = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), hmod, 0);
        if kb.is_err() || ms.is_err() {
            eprintln!("hooks: failed to install (kb={kb:?}, mouse={ms:?})");
        }
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let k = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        let down = matches!(wparam.0 as u32, WM_KEYDOWN | WM_SYSKEYDOWN);
        let ours = input::is_ours(k.dwExtraInfo);
        if down && !ours && k.vkCode == Key::ESCAPE.0 as u32 && PICKING.load(Ordering::SeqCst) && finish_pick(None) {
            return LRESULT(1);
        }
        publish(InputEvent {
            kind: InputKind::Key {
                key: Key(k.vkCode as u16),
                scan: k.scanCode as u16,
                extended: k.flags.contains(LLKHF_EXTENDED),
                down,
            },
            x: 0,
            y: 0,
            ours,
            foreign_injected: !ours && k.flags.contains(LLKHF_INJECTED),
            at: Instant::now(),
        });
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let m = unsafe { &*(lparam.0 as *const MSLLHOOKSTRUCT) };
        let msg = wparam.0 as u32;
        if !input::is_ours(m.dwExtraInfo) {
            if msg == WM_LBUTTONDOWN && PICKING.load(Ordering::SeqCst) && finish_pick(Some((m.pt.x, m.pt.y))) {
                SWALLOW_LEFT_UP.store(true, Ordering::SeqCst);
                return LRESULT(1);
            }
            if msg == WM_LBUTTONUP && SWALLOW_LEFT_UP.swap(false, Ordering::SeqCst) {
                return LRESULT(1);
            }
        }
        let hi = (m.mouseData >> 16) as u16;
        let xbtn = if hi == XBUTTON1 { MouseButton::X1 } else { MouseButton::X2 };
        let kind = match wparam.0 as u32 {
            WM_MOUSEMOVE => {
                let h = HUB.get();
                if h.is_none_or(|h| h.move_subs.load(Ordering::Relaxed) == 0) {
                    return unsafe { CallNextHookEx(None, code, wparam, lparam) };
                }
                Some(InputKind::Move)
            }
            WM_LBUTTONDOWN => Some(InputKind::Button { button: MouseButton::Left, down: true }),
            WM_LBUTTONUP => Some(InputKind::Button { button: MouseButton::Left, down: false }),
            WM_RBUTTONDOWN => Some(InputKind::Button { button: MouseButton::Right, down: true }),
            WM_RBUTTONUP => Some(InputKind::Button { button: MouseButton::Right, down: false }),
            WM_MBUTTONDOWN => Some(InputKind::Button { button: MouseButton::Middle, down: true }),
            WM_MBUTTONUP => Some(InputKind::Button { button: MouseButton::Middle, down: false }),
            WM_XBUTTONDOWN => Some(InputKind::Button { button: xbtn, down: true }),
            WM_XBUTTONUP => Some(InputKind::Button { button: xbtn, down: false }),
            WM_MOUSEWHEEL => Some(InputKind::Wheel { amount: hi as i16 as i32, horizontal: false }),
            WM_MOUSEHWHEEL => Some(InputKind::Wheel { amount: hi as i16 as i32, horizontal: true }),
            _ => None,
        };
        if let Some(kind) = kind {
            let ours = input::is_ours(m.dwExtraInfo);
            publish(InputEvent {
                kind,
                x: m.pt.x,
                y: m.pt.y,
                ours,
                foreign_injected: !ours && (m.flags & LLMHF_INJECTED) != 0,
                at: Instant::now(),
            });
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}
