//! Multi-monitor sanity checks: capture works on every monitor (including
//! ones at negative coordinates) and returns real pixels.
//!
//!   cargo run --example monitors

use engine::capture::{Region, capture};
use engine::input;

fn main() {
    input::ensure_dpi_aware();
    let (vx, vy, vw, vh) = input::virtual_screen();
    println!("virtual screen: {vw}x{vh} at ({vx},{vy})");
    // Sample a 200x100 box near the middle of the left and right halves.
    for (name, x) in [("left half", vx + vw / 4), ("right half", vx + vw * 3 / 4)] {
        let r = Region { x: x - 100, y: vy + vh / 2 - 50, w: 200, h: 100 };
        match capture(r) {
            Ok(f) => {
                let avg = f.bgra.chunks_exact(4).map(|p| p[0] as u64 + p[1] as u64 + p[2] as u64).sum::<u64>() / (3 * 200 * 100);
                let distinct = f.bgra.chunks_exact(4).map(|p| (p[0], p[1], p[2])).collect::<std::collections::HashSet<_>>().len();
                println!("{name}: captured at ({},{}) avg brightness {avg}, {distinct} distinct colours", r.x, r.y);
            }
            Err(e) => println!("{name}: ERROR {e}"),
        }
    }
}
