//! Admin mode. Windows blocks input from a normal process into an elevated
//! (admin) window, so automating an elevated app requires running elevated too.

use windows::Win32::UI::Shell::{IsUserAnAdmin, ShellExecuteW};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows::core::{HSTRING, w};

pub fn is_elevated() -> bool {
    unsafe { IsUserAnAdmin().as_bool() }
}

/// Launch a new elevated copy of this exe (shows the UAC prompt).
/// The caller should exit afterwards on success.
pub fn relaunch_as_admin() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let r = unsafe { ShellExecuteW(None, w!("runas"), &HSTRING::from(exe.as_os_str()), None, None, SW_SHOWNORMAL) };
    // ShellExecute returns a value > 32 on success.
    if r.0 as usize > 32 { Ok(()) } else { Err("Elevation was cancelled".into()) }
}
