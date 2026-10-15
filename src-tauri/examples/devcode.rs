//! Make Dev tab access codes.
//!
//!   cargo run -p hesytools-next --example devcode -- new-key   (once)
//!   cargo run -p hesytools-next --example devcode -- <name>
//!
//! The private key lives at %USERPROFILE%\.tauri\hesytools-dev.key and must
//! never be committed. Losing it only means old codes can't be re-made.

#[path = "../src/dev.rs"]
#[allow(dead_code)]
mod dev;

fn key_path() -> std::path::PathBuf {
    std::path::PathBuf::from(std::env::var("USERPROFILE").expect("USERPROFILE")).join(".tauri").join("hesytools-dev.key")
}

fn main() {
    let arg = std::env::args().nth(1).unwrap_or_default();
    let path = key_path();
    if arg == "new-key" {
        if path.exists() {
            eprintln!("{} already exists; not overwriting it.", path.display());
            std::process::exit(1);
        }
        let mut seed = [0u8; 32];
        getrandom::fill(&mut seed).expect("random");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, seed.iter().map(|b| format!("{b:02x}")).collect::<String>()).unwrap();
        println!("{}", dev::public_key_hex(&seed));
        return;
    }
    if arg.is_empty() || arg.len() > 40 {
        eprintln!("usage: devcode new-key | devcode <name, up to 40 chars>");
        std::process::exit(2);
    }
    let hex = std::fs::read_to_string(&path).expect("no dev key yet: run with new-key first");
    let bytes: Vec<u8> = (0..64).step_by(2).map(|i| u8::from_str_radix(&hex.trim()[i..i + 2], 16).unwrap()).collect();
    let seed: [u8; 32] = bytes.try_into().unwrap();
    println!("{}", dev::make_code(&seed, &arg));
}
