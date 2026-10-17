//! Shut down or restart the PC after a delay, using Windows' own timer
//! (`shutdown.exe`), so it still happens if hesytools is closed and Windows
//! shows its usual warning. With a delay, Windows force-closes other apps.

use std::os::windows::process::CommandExt;
use std::process::Command;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
/// shutdown.exe's maximum delay (10 years).
const MAX_SECS: u64 = 315_360_000;

fn run(args: &[&str]) -> Result<(), String> {
    // Windows' own copy, never a shutdown.exe that happens to sit next to us.
    let windir = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
    let exe = std::path::Path::new(&windir).join("System32").join("shutdown.exe");
    let out = Command::new(exe)
        .args(args)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Couldn't run shutdown: {e}"))?;
    if out.status.success() {
        Ok(())
    } else {
        let msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Err(if msg.is_empty() { format!("shutdown failed ({})", out.status) } else { msg })
    }
}

/// Schedule a shutdown (or restart) in `secs` seconds. Replaces any timer
/// hesytools set before.
pub fn schedule(secs: u64, restart: bool) -> Result<(), String> {
    let _ = cancel();
    let secs = secs.clamp(1, MAX_SECS).to_string();
    let mode = if restart { "/r" } else { "/s" };
    let note = if restart { "hesytools timer: restarting your PC" } else { "hesytools timer: shutting down your PC" };
    run(&[mode, "/t", &secs, "/c", note])
}

/// Cancel a pending shutdown/restart. Errors if none was pending.
pub fn cancel() -> Result<(), String> {
    run(&["/a"])
}
