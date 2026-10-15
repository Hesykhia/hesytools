//! Prints hotkey presses/releases for F6 and Ctrl+Shift+End, for 10 seconds.
//!
//!   cargo run --example hotkeys

use engine::hotkey::HotkeyManager;
use std::time::Duration;

fn main() {
    let mgr = HotkeyManager::start(|id, pressed| {
        println!("{id} {}", if pressed { "pressed" } else { "released" });
    });
    mgr.bind("f6", Some("F6".parse().unwrap()));
    mgr.bind("combo", Some("Ctrl+Shift+End".parse().unwrap()));
    eprintln!("hotkeys: listening for 10s");
    std::thread::sleep(Duration::from_secs(10));
}
