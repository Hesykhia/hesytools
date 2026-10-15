//! Taps F24 every 20 ms for N seconds, only while <exe> is in front, and
//! prints whether the lock allows a mouse action at a few points.
//!   cargo run --example locktest -- powershell.exe 6 [x,y ...]
use engine::applock::AppLock;
use engine::clicker::{ClickAction, Clicker, ClickerConfig, StopAfter};
use engine::keys::Key;
use engine::window::WindowTarget;
fn main() {
    engine::input::ensure_dpi_aware();
    let a: Vec<String> = std::env::args().skip(1).collect();
    let target = WindowTarget { exe: a[0].clone(), title: String::new() };
    let secs: f64 = a[1].parse().unwrap();
    for p in &a[2..] {
        let (x, y) = p.split_once(',').unwrap();
        let lock = AppLock::new(target.clone());
        println!("allows mouse at {p}: {}", lock.allows(Some((x.parse().unwrap(), y.parse().unwrap()))));
    }
    if secs == 0.0 { return; }
    let c = Clicker::new();
    c.start(ClickerConfig { action: ClickAction::Key { key: Key::from_name("F24").unwrap() }, interval_ms: 20.0, hold_ms: 0.0,
        stop_after: StopAfter::Seconds { secs }, only_in: Some(target), ..Default::default() }, |_| {}).unwrap();
    std::thread::sleep(std::time::Duration::from_secs_f64(secs + 0.3));
    println!("clicks sent: {}", c.status().clicks);
}
