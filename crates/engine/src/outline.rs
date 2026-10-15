//! A red outline around every monitor while recording.
//!
//! Each monitor gets a borderless, topmost, layered window whose interior is
//! a colour-key "hole" and which is click-through (WS_EX_TRANSPARENT), so it
//! never blocks the mouse, never takes focus, and can't end up in a
//! recording. The primary monitor also shows a small "REC" label.

use std::thread::JoinHandle;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::{BOOL, w};

const KEY: COLORREF = COLORREF(0x00FF00FF); // magenta = see-through
const RED: COLORREF = COLORREF(0x00705CFF); // BGR of #ff5c70
const THICKNESS: i32 = 4;

pub struct ScreenOutline {
    thread_id: u32,
    thread: Option<JoinHandle<()>>,
}

impl ScreenOutline {
    /// Show the outline. `label` is drawn at the top of the primary monitor.
    pub fn show(label: &str) -> Self {
        let label = label.to_string();
        let (tx, rx) = std::sync::mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("recording-outline".into())
            .spawn(move || {
                let _ = tx.send(unsafe { GetCurrentThreadId() });
                run(&label);
            })
            .expect("spawn outline thread");
        let thread_id = rx.recv().unwrap_or(0);
        Self { thread_id, thread: Some(thread) }
    }
}

impl Drop for ScreenOutline {
    fn drop(&mut self) {
        unsafe {
            let _ = PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
        }
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

thread_local! {
    static LABEL: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
}

fn monitors() -> Vec<(RECT, bool)> {
    unsafe extern "system" fn each(m: HMONITOR, _: HDC, _: *mut RECT, data: LPARAM) -> BOOL {
        let out = unsafe { &mut *(data.0 as *mut Vec<(RECT, bool)>) };
        let mut info = MONITORINFO { cbSize: size_of::<MONITORINFO>() as u32, ..Default::default() };
        if unsafe { GetMonitorInfoW(m, &mut info) }.as_bool() {
            out.push((info.rcMonitor, info.dwFlags & MONITORINFOF_PRIMARY != 0));
        }
        BOOL(1)
    }
    let mut out: Vec<(RECT, bool)> = Vec::new();
    unsafe {
        let _ = EnumDisplayMonitors(None, None, Some(each), LPARAM(&mut out as *mut _ as isize));
    }
    out
}

fn run(label: &str) {
    LABEL.with(|l| *l.borrow_mut() = label.to_string());
    unsafe {
        let Ok(hinst) = GetModuleHandleW(None) else { return };
        let class = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: hinst.into(),
            lpszClassName: w!("hesytools-recording-outline"),
            ..Default::default()
        };
        RegisterClassW(&class);
        let mut windows = Vec::new();
        for (r, primary) in monitors() {
            let Ok(hwnd) = CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                w!("hesytools-recording-outline"),
                w!(""),
                WS_POPUP,
                r.left,
                r.top,
                r.right - r.left,
                r.bottom - r.top,
                None,
                None,
                Some(hinst.into()),
                // Marks the primary monitor's window so it draws the label.
                Some(primary as usize as *const _),
            ) else {
                continue;
            };
            let _ = SetLayeredWindowAttributes(hwnd, KEY, 255, LWA_COLORKEY);
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            windows.push(hwnd);
        }
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            DispatchMessageW(&msg);
        }
        for h in windows {
            let _ = DestroyWindow(h);
        }
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_NCCREATE => {
                let cs = &*(lp.0 as *const CREATESTRUCTW);
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, cs.lpCreateParams as isize);
                DefWindowProcW(hwnd, msg, wp, lp)
            }
            // Click-through even for the few visible border pixels.
            WM_NCHITTEST => LRESULT(HTTRANSPARENT as isize),
            WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
            WM_ERASEBKGND => LRESULT(1),
            WM_PAINT => {
                let mut ps = PAINTSTRUCT::default();
                let hdc = BeginPaint(hwnd, &mut ps);
                let mut rc = RECT::default();
                let _ = GetClientRect(hwnd, &mut rc);
                let hole = CreateSolidBrush(KEY);
                FillRect(hdc, &rc, hole);
                let _ = DeleteObject(hole.into());
                let red = CreateSolidBrush(RED);
                for edge in [
                    RECT { bottom: THICKNESS, ..rc },
                    RECT { top: rc.bottom - THICKNESS, ..rc },
                    RECT { right: THICKNESS, ..rc },
                    RECT { left: rc.right - THICKNESS, ..rc },
                ] {
                    FillRect(hdc, &edge, red);
                }
                if GetWindowLongPtrW(hwnd, GWLP_USERDATA) != 0 {
                    draw_label(hdc, &rc, red);
                }
                let _ = DeleteObject(red.into());
                let _ = EndPaint(hwnd, &ps);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wp, lp),
        }
    }
}

unsafe fn draw_label(hdc: HDC, rc: &RECT, red: HBRUSH) {
    let text = LABEL.with(|l| l.borrow().clone());
    if text.is_empty() {
        return;
    }
    unsafe {
        let font = CreateFontW(
            -15, 0, 0, 0, 650, 0, 0, 0, DEFAULT_CHARSET, OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY, 0, w!("Segoe UI"),
        );
        let old = SelectObject(hdc, font.into());
        let wide: Vec<u16> = text.encode_utf16().collect();
        let mut size = SIZE::default();
        let _ = GetTextExtentPoint32W(hdc, &wide, &mut size);
        let (w, h) = (size.cx + 46, 30);
        let x = (rc.right - w) / 2;
        let pill = RECT { left: x, top: THICKNESS, right: x + w, bottom: THICKNESS + h };
        FillRect(hdc, &pill, red);
        // White dot, then the text.
        let white = CreateSolidBrush(COLORREF(0x00FFFFFF));
        let old_brush = SelectObject(hdc, white.into());
        let old_pen = SelectObject(hdc, GetStockObject(NULL_PEN));
        let _ = Ellipse(hdc, x + 14, pill.top + 11, x + 23, pill.top + 20);
        SelectObject(hdc, old_pen);
        SelectObject(hdc, old_brush);
        let _ = DeleteObject(white.into());
        SetBkMode(hdc, TRANSPARENT);
        SetTextColor(hdc, COLORREF(0x00FFFFFF));
        let _ = TextOutW(hdc, x + 32, pill.top + (h - size.cy) / 2, &wide);
        SelectObject(hdc, old);
        let _ = DeleteObject(font.into());
    }
}
