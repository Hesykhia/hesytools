//! Admin mode. Windows blocks input from a normal process into an elevated
//! (admin) window, so automating an elevated app requires running elevated too.

use windows::Win32::UI::Shell::{IsUserAnAdmin, ShellExecuteW};
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
use windows::core::{HSTRING, w};

pub fn is_elevated() -> bool {
    unsafe { IsUserAnAdmin().as_bool() }
}

const RELAUNCH_ARG: &str = "--relaunched-from=";

/// Launch a new elevated copy of this exe (shows the UAC prompt).
/// The caller should exit afterwards on success.
pub fn relaunch_as_admin() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let args = HSTRING::from(format!("{RELAUNCH_ARG}{}", std::process::id()));
    let r = unsafe {
        ShellExecuteW(None, w!("runas"), &HSTRING::from(exe.as_os_str()), &args, None, SW_SHOWNORMAL)
    };
    // ShellExecute returns a value > 32 on success.
    if r.0 as usize > 32 { Ok(()) } else { Err("Elevation was cancelled".into()) }
}

/// If this process was started by [`relaunch_as_admin`], wait (up to 5s) for
/// the old instance to exit so single-instance checks don't kill us.
pub fn wait_for_previous_instance() {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_SYNCHRONIZE, WaitForSingleObject};
    let Some(pid) = std::env::args()
        .find_map(|a| a.strip_prefix(RELAUNCH_ARG).and_then(|p| p.parse::<u32>().ok()))
    else {
        return;
    };
    unsafe {
        if let Ok(h) = OpenProcess(PROCESS_SYNCHRONIZE, false, pid) {
            WaitForSingleObject(h, 5000);
            let _ = CloseHandle(h);
        }
    }
}
