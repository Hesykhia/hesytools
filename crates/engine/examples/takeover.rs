//! Binds F24 as a hotkey for a few seconds, then unbinds it. While bound,
//! presses should fire the hotkey and be swallowed (other apps don't see
//! them); after unbinding they pass through again.
//!
//!   cargo run --example takeover

use std::time::Duration;

fn main() {
    let mgr = engine::hotkey::HotkeyManager::start(|id, pressed| {
        if pressed {
            println!("hotkey fired: {id}");
        }
    });
    mgr.bind("test", Some("F24".parse().unwrap()));
    eprintln!("takeover: F24 bound for 3s");
    std::thread::sleep(Duration::from_secs(3));
    mgr.bind("test", None);
    println!("unbound");
    std::thread::sleep(Duration::from_secs(3));
}
