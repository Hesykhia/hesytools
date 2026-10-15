//! Plays a macro that waits for text in a region, then taps F24, and reports
//! how long the wait took.
//!
//!   cargo run --release --example waitfor -- <x> <y> <w> <h> <text>

use engine::capture::Region;
use engine::input;
use engine::keys::Key;
use engine::macros::{self, Macro, OnTimeout, PressAction, Step};
use engine::timer::Waiter;
use engine::watch::Condition;
use std::time::Instant;

fn main() {
    input::ensure_dpi_aware();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let n = |i: usize| args[i].parse::<i64>().unwrap();
    let region = Region { x: n(0) as i32, y: n(1) as i32, w: n(2) as u32, h: n(3) as u32 };
    let m = Macro {
        steps: vec![
            Step::WaitFor {
                region,
                condition: Condition::TextAppears { text: args[4].clone(), digits_after: None },
                timeout_ms: 8000.0,
                poll_ms: 100.0,
                on_timeout: OnTimeout::Stop,
                window: None,
            },
            Step::Key { key: Key::from_name("F24").unwrap(), action: PressAction::Tap, hold_ms: 10.0 },
        ],
        ..Default::default()
    };
    let t = Instant::now();
    let r = macros::play(&m, &Waiter::new().unwrap(), |i, _| {
        if i == 1 {
            println!("text found after {:.0} ms, pressing F24", t.elapsed().as_secs_f64() * 1000.0);
        }
    });
    println!("finished={} error={:?}", r.finished, r.error);
}
