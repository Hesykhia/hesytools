//! Opens the drag-to-select overlay and prints the chosen region.
//!
//!   cargo run --example pickregion

fn main() {
    engine::input::ensure_dpi_aware();
    println!("{:?}", engine::picker::pick_region());
}
