//! Watches a region and reports what OCR reads, how long it takes, and how
//! often OCR actually had to run.
//!
//!   cargo run --release --example ocrtest -- <x> <y> <w> <h> [checks]

use engine::capture::Region;
use engine::input;
use engine::watch::{Cmp, Condition, Watcher};
use std::time::{Duration, Instant};

fn main() {
    input::ensure_dpi_aware();
    let a: Vec<i64> = std::env::args().skip(1).filter_map(|s| s.parse().ok()).collect();
    let region = Region { x: a[0] as i32, y: a[1] as i32, w: a[2] as u32, h: a[3] as u32 };
    let checks = a.get(4).copied().unwrap_or(30) as usize;
    let mut w = Watcher::new(region, Condition::Number { op: Cmp::AtLeast, value: 1e25 }).unwrap();

    let cpu0 = process_cpu();
    let t0 = Instant::now();
    let (mut ocr_runs, mut first_ocr_ms, mut cap_total) = (0, 0.0, 0.0);
    let mut last = None;
    for i in 0..checks {
        let c = w.check().unwrap();
        cap_total += c.capture_ms;
        if c.ocr_ran {
            ocr_runs += 1;
            if i == 0 {
                first_ocr_ms = c.ocr_ms;
            }
        }
        last = Some(c);
        std::thread::sleep(Duration::from_millis(100));
    }
    let c = last.unwrap();
    let cpu = (process_cpu() - cpu0).as_secs_f64() / t0.elapsed().as_secs_f64() * 100.0;
    println!("read:       {:?}", c.text.unwrap_or_default());
    println!("numbers:    {:?}   matched (>= 1e25): {}  (by {:?})", c.numbers, c.matched, c.number);
    println!("checks: {checks}  ocr runs: {ocr_runs}  first ocr: {first_ocr_ms:.1}ms  avg capture: {:.2}ms", cap_total / checks as f64);
    println!("cpu while watching every 100ms: {cpu:.2}% of one core");
}

fn process_cpu() -> Duration {
    use windows::Win32::Foundation::FILETIME;
    use windows::Win32::System::Threading::{GetCurrentProcess, GetProcessTimes};
    let (mut c, mut e, mut k, mut u) = Default::default();
    unsafe { GetProcessTimes(GetCurrentProcess(), &mut c, &mut e, &mut k, &mut u).unwrap() };
    let t = |f: FILETIME| ((f.dwHighDateTime as u64) << 32 | f.dwLowDateTime as u64) * 100;
    Duration::from_nanos(t(k) + t(u))
}
