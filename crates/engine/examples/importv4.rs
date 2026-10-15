//! Converts an old hesytools v4 macro file and prints the steps.
//!
//!   cargo run --example importv4 -- path/to/macro.json

fn main() {
    let path = std::env::args().nth(1).expect("path to a v4 macro json");
    let json = std::fs::read_to_string(&path).expect("read file");
    match engine::import::import_v4(&json, "imported") {
        Ok(m) => {
            println!("hotkey={:?} speed={} steps={}", m.hotkey.map(|h| h.to_string()), m.speed, m.steps.len());
            for s in &m.steps {
                println!("  {}", serde_json::to_string(s).unwrap());
            }
        }
        Err(e) => println!("error: {e}"),
    }
}
