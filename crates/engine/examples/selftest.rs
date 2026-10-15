//! Measures what the engine actually does on this machine.
//!
//!   cargo run --release --example selftest
//!
//! Only presses F24 (no physical keyboard has it, so nothing gets typed) and
//! briefly moves the cursor, restoring it afterwards.

use engine::clicker::{ClickAction, Clicker, ClickerConfig, StopAfter};
use engine::hooks::{self, InputKind};
use engine::input;
use engine::keys::Key;
use std::time::{Duration, Instant};

fn main() {
    input::ensure_dpi_aware();
    let f24 = Key::from_name("F24").unwrap();
    let rx = hooks::subscribe(false);
    std::thread::sleep(Duration::from_millis(200)); // let hooks install

    // 1. Our events reach the system as scancodes, tagged as ours.
    while rx.try_recv().is_ok() {}
    input::tap_key(f24);
    let mut seen = None;
    let deadline = Instant::now() + Duration::from_secs(1);
    while let Ok(ev) = rx.recv_deadline(deadline) {
        if let InputKind::Key { key, scan, down: true, .. } = ev.kind {
            if key == f24 {
                seen = Some((scan, ev.ours));
                break;
            }
        }
    }
    match seen {
        Some((scan, ours)) => println!("[keys]   F24 seen by hooks: scan=0x{scan:02X} tagged-as-ours={ours}"),
        None => println!("[keys]   FAIL: F24 never arrived"),
    }

    // 2. Absolute moves land on the exact pixel, including other monitors.
    let (vx, vy, vw, vh) = input::virtual_screen();
    let saved = input::cursor_pos();
    let points = [
        (vx, vy),
        (vx + vw - 1, vy + vh - 1),
        (vx + vw / 3, vy + vh / 2),
        (saved.0 + 7, saved.1 + 3),
    ];
    let mut misses = 0;
    for (x, y) in points {
        input::move_abs(x, y);
        std::thread::sleep(Duration::from_millis(15));
        let got = input::cursor_pos();
        if got != (x, y) {
            misses += 1;
            println!("[move]   wanted ({x},{y}) got {got:?}");
        }
    }
    input::move_abs(saved.0, saved.1);
    println!(
        "[move]   virtual screen {vw}x{vh} at ({vx},{vy}): {}/{} exact",
        points.len() - misses,
        points.len()
    );

    // 3. Clicker timing + CPU cost at several rates.
    println!("[timing] interval   clicks   mean      p50       p99       max       cpu");
    for interval in [1.0, 5.0, 16.0, 50.0] {
        let secs = 2.0;
        let clicker = Clicker::new();
        while rx.try_recv().is_ok() {}
        let cpu0 = process_cpu();
        let wall0 = Instant::now();
        clicker
            .start(ClickerConfig {
                action: ClickAction::Key { key: f24 },
                interval_ms: interval,
                hold_ms: 0.0,
                stop_after: StopAfter::Seconds { secs },
                ..Default::default()
            }, |_| {})
            .unwrap();
        let mut downs = Vec::new();
        let end = Instant::now() + Duration::from_secs_f64(secs + 0.3);
        while let Ok(ev) = rx.recv_deadline(end) {
            if let InputKind::Key { key, down: true, .. } = ev.kind {
                if key == f24 {
                    downs.push(ev.at);
                }
            }
        }
        clicker.stop();
        let cpu = (process_cpu() - cpu0).as_secs_f64() / wall0.elapsed().as_secs_f64() * 100.0;
        let mut gaps: Vec<f64> = downs.windows(2).map(|w| (w[1] - w[0]).as_secs_f64() * 1000.0).collect();
        gaps.sort_by(f64::total_cmp);
        let pct = |p: f64| gaps.get(((gaps.len() as f64 - 1.0) * p) as usize).copied().unwrap_or(0.0);
        let mean = gaps.iter().sum::<f64>() / gaps.len().max(1) as f64;
        println!(
            "[timing] {interval:>5.1}ms  {:>7}  {mean:7.3}ms {:7.3}ms {:7.3}ms {:7.3}ms  {cpu:4.1}% of one core",
            downs.len(),
            pct(0.5),
            pct(0.99),
            pct(1.0)
        );
    }
}

fn process_cpu() -> Duration {
    use windows::Win32::Foundation::FILETIME;
    use windows::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};
    let (mut c, mut e, mut k, mut u) = Default::default();
    unsafe { GetProcessTimes(GetCurrentProcess(), &mut c, &mut e, &mut k, &mut u).unwrap() };
    let t = |f: FILETIME| ((f.dwHighDateTime as u64) << 32 | f.dwLowDateTime as u64) * 100;
    Duration::from_nanos(t(k) + t(u))
}
