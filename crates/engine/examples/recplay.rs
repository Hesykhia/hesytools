//! Records input for N seconds, prints the steps, then replays them and
//! reports when each replayed key landed.
//!
//!   cargo run --example recplay -- 4
//!
//! Only meant to be driven by injected F24 presses (see the test scripts);
//! replaying real typing would type it again.

use engine::hooks::{self, InputKind};
use engine::macros::{self, Macro, RecordOptions, Recorder, Step};
use engine::timer::Waiter;
use std::time::{Duration, Instant};

fn main() {
    let secs: u64 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(4);
    let rec = Recorder::start(RecordOptions::default(), |_| true);
    eprintln!("recplay: recording for {secs}s");
    std::thread::sleep(Duration::from_secs(secs));
    let steps = rec.stop();
    println!("recorded {} steps: {}", steps.len(), serde_json::to_string(&steps).unwrap());

    if steps.iter().any(|s| matches!(s, Step::Key { key, .. } if key.name() != "F24")) {
        eprintln!("recplay: recording contains keys other than F24; not replaying");
        return;
    }
    let rx = hooks::subscribe(false);
    std::thread::sleep(Duration::from_millis(100));
    let m = Macro { steps, ..Default::default() };
    let start = Instant::now();
    let r = macros::play(&m, &Waiter::new().unwrap(), |_, _| {});
    let mut downs = Vec::new();
    while let Ok(ev) = rx.recv_timeout(Duration::from_millis(200)) {
        if let InputKind::Key { down: true, key, .. } = ev.kind {
            if ev.ours {
                downs.push(format!("{}@{:.0}ms", key, (ev.at - start).as_secs_f64() * 1000.0));
            }
        }
    }
    println!("replayed: finished={} -> {}", r.finished, downs.join(", "));
}
