//! "Start with Windows": a per-user entry under
//! HKCU\Software\Microsoft\Windows\CurrentVersion\Run (no admin needed).

use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::System::Registry::*;
use windows::core::{PCWSTR, w};

const RUN_KEY: PCWSTR = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
const VALUE: PCWSTR = w!("hesytools");

/// Passed when launched by Windows at sign-in, so we start hidden in the tray.
pub const STARTUP_ARG: &str = "--startup";

pub fn is_enabled() -> bool {
    let mut size = 0u32;
    unsafe {
        RegGetValueW(HKEY_CURRENT_USER, RUN_KEY, VALUE, RRF_RT_REG_SZ, None, None, Some(&mut size)) == ERROR_SUCCESS
    }
}

pub fn set_enabled(on: bool) -> Result<(), String> {
    unsafe {
        let mut key = HKEY::default();
        let r = RegCreateKeyExW(HKEY_CURRENT_USER, RUN_KEY, None, None, REG_OPTION_NON_VOLATILE, KEY_SET_VALUE, None, &mut key, None);
        if r != ERROR_SUCCESS {
            return Err(format!("Couldn't open the startup list ({r:?})"));
        }
        let result = if on {
            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
            let cmd = format!("\"{}\" {STARTUP_ARG}", exe.display());
            let wide: Vec<u16> = cmd.encode_utf16().chain(std::iter::once(0)).collect();
            let bytes = std::slice::from_raw_parts(wide.as_ptr() as *const u8, wide.len() * 2);
            RegSetValueExW(key, VALUE, None, REG_SZ, Some(bytes))
        } else {
            match RegDeleteValueW(key, VALUE) {
                r if r == windows::Win32::Foundation::ERROR_FILE_NOT_FOUND => ERROR_SUCCESS,
                r => r,
            }
        };
        let _ = RegCloseKey(key);
        if result == ERROR_SUCCESS { Ok(()) } else { Err(format!("Couldn't update the startup list ({result:?})")) }
    }
}

pub fn launched_at_startup() -> bool {
    std::env::args().any(|a| a == STARTUP_ARG)
}
