//! Shows the recording outline for a few seconds and checks that it's
//! click-through (the window under a border pixel isn't the outline).
//!
//!   cargo run --example outline

use engine::input;
use std::time::Duration;

fn main() {
    input::ensure_dpi_aware();
    let before = input::root_window_at(2, 300);
    let o = engine::outline::ScreenOutline::show("REC  ·  press F9 to stop");
    std::thread::sleep(Duration::from_millis(600));
    let during = input::root_window_at(2, 300);
    println!("window under left border: before={before:#x} during={during:#x} click-through={}", before == during);
    std::thread::sleep(Duration::from_millis(2400));
    drop(o);
    println!("closed");
}
