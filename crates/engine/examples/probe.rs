//! Shows input exactly as a game reading Raw Input receives it.
//!
//!   cargo run --example probe            # live log of every event
//!   cargo run --example probe -- 10      # stop after 10 seconds
//!
//! Keyboard lines show the hardware scancode. If it's 0, the sender used
//! virtual keys only and many games will ignore it.

use std::sync::OnceLock;
use std::time::Instant;
use windows::Win32::Foundation::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::w;

static START: OnceLock<Instant> = OnceLock::new();

fn main() {
    let secs: Option<u64> = std::env::args().nth(1).and_then(|s| s.parse().ok());
    START.get_or_init(Instant::now);
    unsafe {
        let hinst = GetModuleHandleW(None).unwrap();
        let class = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: hinst.into(),
            lpszClassName: w!("hesytools-probe"),
            ..Default::default()
        };
        RegisterClassW(&class);
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("hesytools-probe"),
            w!("probe"),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            Some(hinst.into()),
            None,
        )
        .expect("create window");
        let devices = [
            RAWINPUTDEVICE { usUsagePage: 1, usUsage: 6, dwFlags: RIDEV_INPUTSINK, hwndTarget: hwnd },
            RAWINPUTDEVICE { usUsagePage: 1, usUsage: 2, dwFlags: RIDEV_INPUTSINK, hwndTarget: hwnd },
        ];
        RegisterRawInputDevices(&devices, size_of::<RAWINPUTDEVICE>() as u32).expect("register raw input");
        if let Some(s) = secs {
            SetTimer(Some(hwnd), 1, (s * 1000) as u32, None);
        }
        eprintln!("probe: listening for raw input{}", secs.map(|s| format!(" for {s}s")).unwrap_or_default());
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            DispatchMessageW(&msg);
        }
    }
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_INPUT => {
                let mut raw = RAWINPUT::default();
                let mut size = size_of::<RAWINPUT>() as u32;
                let n = GetRawInputData(
                    HRAWINPUT(lp.0 as _),
                    RID_INPUT,
                    Some(&mut raw as *mut _ as _),
                    &mut size,
                    size_of::<RAWINPUTHEADER>() as u32,
                );
                if n != u32::MAX {
                    let t = START.get().unwrap().elapsed().as_secs_f64() * 1000.0;
                    if raw.header.dwType == RIM_TYPEKEYBOARD.0 {
                        let k = raw.data.keyboard;
                        let up = k.Flags & RI_KEY_BREAK as u16 != 0;
                        let e0 = k.Flags & RI_KEY_E0 as u16 != 0;
                        println!(
                            "{t:10.3}ms KEY   {} vk=0x{:02X} scan=0x{:02X}{}",
                            if up { "up  " } else { "down" },
                            k.VKey,
                            k.MakeCode,
                            if e0 { " (E0 extended)" } else { "" }
                        );
                    } else if raw.header.dwType == RIM_TYPEMOUSE.0 {
                        let m = raw.data.mouse;
                        let btn = m.Anonymous.Anonymous.usButtonFlags;
                        let abs = m.usFlags.0 & MOUSE_MOVE_ABSOLUTE.0 != 0;
                        println!(
                            "{t:10.3}ms MOUSE buttons=0x{btn:04X} {} dx={} dy={}",
                            if abs { "abs" } else { "rel" },
                            m.lLastX,
                            m.lLastY
                        );
                    }
                }
                DefWindowProcW(hwnd, msg, wp, lp)
            }
            WM_TIMER => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wp, lp),
        }
    }
}
