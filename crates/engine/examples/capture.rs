//! Waits for the next hotkey press and prints how it was captured.
//!
//!   cargo run --example capture

fn main() {
    let mgr = engine::hotkey::HotkeyManager::start(|_, _| {});
    std::thread::sleep(std::time::Duration::from_millis(300));
    eprintln!("capture: press a key");
    println!("{:?}", mgr.capture_next().map(|h| h.to_string()));
}
