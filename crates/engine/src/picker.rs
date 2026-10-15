//! Drag-to-select a screen region, like the Snipping Tool.
//!
//! A borderless, topmost, semi-transparent window covers every monitor.
//! The selection itself is painted in a colour-key colour so it's fully
//! see-through: you see exactly what will be read.

use crate::capture::Region;
use crate::input;
use std::cell::RefCell;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture, SetFocus};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::w;

const KEY: COLORREF = COLORREF(0x00FF00FF); // magenta = transparent hole
const DIM_ALPHA: u8 = 120;
const ACCENT: COLORREF = COLORREF(0x00FF6C8B); // BGR of #8b6cff

#[derive(Default)]
struct State {
    origin: (i32, i32),
    start: Option<(i32, i32)>,
    current: (i32, i32),
    done: Option<Option<Region>>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::default());
}

/// Blocks until the user drags a rectangle (Some) or presses Esc /
/// right-clicks (None). Runs its own window on a fresh thread.
pub fn pick_region() -> Option<Region> {
    std::thread::spawn(run).join().ok().flatten()
}

fn run() -> Option<Region> {
    let (vx, vy, vw, vh) = input::virtual_screen();
    STATE.with(|s| *s.borrow_mut() = State { origin: (vx, vy), ..Default::default() });
    unsafe {
        let hinst = GetModuleHandleW(None).ok()?;
        let class = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: hinst.into(),
            lpszClassName: w!("hesytools-region-picker"),
            hCursor: LoadCursorW(None, IDC_CROSS).ok()?,
            ..Default::default()
        };
        RegisterClassW(&class);
        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_LAYERED,
            w!("hesytools-region-picker"),
            w!("Select a region"),
            WS_POPUP,
            vx,
            vy,
            vw,
            vh,
            None,
            None,
            Some(hinst.into()),
            None,
        )
        .ok()?;
        let _ = SetLayeredWindowAttributes(hwnd, KEY, DIM_ALPHA, LWA_ALPHA | LWA_COLORKEY);
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);
        let _ = SetFocus(Some(hwnd));

        let mut msg = MSG::default();
        while STATE.with(|s| s.borrow().done.is_none()) && GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        let _ = DestroyWindow(hwnd);
    }
    STATE.with(|s| s.borrow_mut().done.take().flatten())
}

fn selection(s: &State) -> Option<RECT> {
    let (ax, ay) = s.start?;
    let (bx, by) = s.current;
    Some(RECT { left: ax.min(bx), top: ay.min(by), right: ax.max(bx), bottom: ay.max(by) })
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let pt = || ((lp.0 & 0xFFFF) as i16 as i32, ((lp.0 >> 16) & 0xFFFF) as i16 as i32);
    unsafe {
        match msg {
            WM_LBUTTONDOWN => {
                let p = pt();
                STATE.with(|s| {
                    let mut s = s.borrow_mut();
                    s.start = Some(p);
                    s.current = p;
                });
                SetCapture(hwnd);
                LRESULT(0)
            }
            WM_MOUSEMOVE => {
                let p = pt();
                let dragging = STATE.with(|s| {
                    let mut s = s.borrow_mut();
                    s.current = p;
                    s.start.is_some()
                });
                if dragging {
                    let _ = InvalidateRect(Some(hwnd), None, false);
                }
                LRESULT(0)
            }
            WM_LBUTTONUP => {
                let _ = ReleaseCapture();
                STATE.with(|s| {
                    let mut s = s.borrow_mut();
                    s.current = pt();
                    let (ox, oy) = s.origin;
                    let region = selection(&s).filter(|r| r.right - r.left >= 4 && r.bottom - r.top >= 4).map(|r| Region {
                        x: r.left + ox,
                        y: r.top + oy,
                        w: (r.right - r.left) as u32,
                        h: (r.bottom - r.top) as u32,
                    });
                    // A tiny accidental drag just restarts the selection.
                    if region.is_some() {
                        s.done = Some(region);
                    } else {
                        s.start = None;
                    }
                });
                let _ = InvalidateRect(Some(hwnd), None, false);
                LRESULT(0)
            }
            WM_RBUTTONDOWN => {
                STATE.with(|s| s.borrow_mut().done = Some(None));
                LRESULT(0)
            }
            WM_KEYDOWN if wp.0 == 0x1B => {
                STATE.with(|s| s.borrow_mut().done = Some(None));
                LRESULT(0)
            }
            WM_ERASEBKGND => LRESULT(1),
            WM_PAINT => {
                let mut ps = PAINTSTRUCT::default();
                let hdc = BeginPaint(hwnd, &mut ps);
                let mut rc = RECT::default();
                let _ = GetClientRect(hwnd, &mut rc);
                // Double-buffer so dragging doesn't flicker.
                let mem = CreateCompatibleDC(Some(hdc));
                let bmp = CreateCompatibleBitmap(hdc, rc.right, rc.bottom);
                let old = SelectObject(mem, bmp.into());
                let bg = CreateSolidBrush(COLORREF(0x00140A0A));
                FillRect(mem, &rc, bg);
                let _ = DeleteObject(bg.into());

                let sel = STATE.with(|s| selection(&s.borrow()));
                if let Some(r) = sel {
                    let hole = CreateSolidBrush(KEY);
                    FillRect(mem, &r, hole);
                    let _ = DeleteObject(hole.into());
                    let pen = CreatePen(PS_SOLID, 2, ACCENT);
                    let oldpen = SelectObject(mem, pen.into());
                    let oldbr = SelectObject(mem, GetStockObject(NULL_BRUSH));
                    let _ = Rectangle(mem, r.left - 1, r.top - 1, r.right + 1, r.bottom + 1);
                    SelectObject(mem, oldbr);
                    SelectObject(mem, oldpen);
                    let _ = DeleteObject(pen.into());
                    draw_label(mem, &format!("{} × {}", r.right - r.left, r.bottom - r.top), r.left, (r.top - 26).max(4));
                }
                draw_label(mem, "Drag to select the area to watch  ·  Esc or right-click to cancel", rc.right / 2 - 230, 24);

                let _ = BitBlt(hdc, 0, 0, rc.right, rc.bottom, Some(mem), 0, 0, SRCCOPY);
                SelectObject(mem, old);
                let _ = DeleteObject(bmp.into());
                let _ = DeleteDC(mem);
                let _ = EndPaint(hwnd, &ps);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wp, lp),
        }
    }
}

unsafe fn draw_label(dc: HDC, text: &str, x: i32, y: i32) {
    unsafe {
        SetBkMode(dc, TRANSPARENT);
        SetTextColor(dc, COLORREF(0x00FFFFFF));
        let font = CreateFontW(
            -18, 0, 0, 0, 600, 0, 0, 0, DEFAULT_CHARSET, OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY, 0, w!("Segoe UI"),
        );
        let old = SelectObject(dc, font.into());
        let wide: Vec<u16> = text.encode_utf16().collect();
        let _ = TextOutW(dc, x, y, &wide);
        SelectObject(dc, old);
        let _ = DeleteObject(font.into());
    }
}
