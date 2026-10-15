//! Lists app windows; with an exe name, captures a box from that window and
//! the same box straight off the screen, as BMP files, to compare.
//!
//!   cargo run --example wincap -- [exe] [x y w h] [out-dir]
use engine::capture::{self, Frame, Region};
use engine::window::{self, WindowTarget};

fn save_bmp(f: &Frame, path: &str) {
    let mut out = Vec::new();
    let size = 54 + f.bgra.len() as u32;
    out.extend_from_slice(b"BM");
    for v in [size, 0, 54, 40] { out.extend_from_slice(&v.to_le_bytes()); }
    out.extend_from_slice(&(f.w as i32).to_le_bytes());
    out.extend_from_slice(&(-(f.h as i32)).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    for _ in 0..6 { out.extend_from_slice(&0u32.to_le_bytes()); }
    out.extend_from_slice(&f.bgra);
    std::fs::write(path, out).unwrap();
}

fn main() {
    engine::input::ensure_dpi_aware();
    let a: Vec<String> = std::env::args().skip(1).collect();
    if a.is_empty() {
        for w in window::list().iter().take(15) {
            println!("{:28} {:5},{:5} {:4}x{:4}  {}", w.exe, w.x, w.y, w.w, w.h, w.title.chars().take(50).collect::<String>());
        }
        return;
    }
    let n = |i: usize| a[i].parse::<i32>().unwrap();
    let r = Region { x: n(1), y: n(2), w: n(3) as u32, h: n(4) as u32 };
    let dir = &a[5];
    let h = window::find(&WindowTarget { exe: a[0].clone(), title: String::new() }).expect("window not found");
    let t = std::time::Instant::now();
    let (f, origin) = capture::capture_window(h, r).unwrap();
    println!("window capture {:.1} ms, region on screen at {origin:?}", t.elapsed().as_secs_f64() * 1000.0);
    save_bmp(&f, &format!("{dir}/win.bmp"));
    let s = capture::capture(Region { x: origin.0, y: origin.1, w: r.w, h: r.h }).unwrap();
    save_bmp(&s, &format!("{dir}/screen.bmp"));
}
