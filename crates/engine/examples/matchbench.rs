//! Times image matching on a realistic size: 60x40 icon in a 400x300 area.
//!
//!   cargo run --release --example matchbench

use engine::imagematch::{Gray, find};
use std::time::Instant;

fn main() {
    let (w, h) = (400u32, 300u32);
    let px = (0..w * h).map(|i| ((i % w) * 7 ^ (i / w) * 13).wrapping_mul(2654435761) as u8).collect();
    let hay = Gray { w, h, px };
    let mut tpx = Vec::new();
    for y in 150..190 {
        tpx.extend_from_slice(&hay.px[(y * w + 250) as usize..(y * w + 310) as usize]);
    }
    let tpl = Gray { w: 60, h: 40, px: tpx };
    let t = Instant::now();
    let runs = 50;
    let mut f = None;
    for _ in 0..runs {
        f = find(&hay, &tpl);
    }
    println!("{:?} in {:.2} ms per search", f, t.elapsed().as_secs_f64() * 1000.0 / runs as f64);
}
