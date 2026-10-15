//! Schedules a shutdown an hour out and cancels it straight away. Cancel
//! only succeeds if one was really pending, so this checks both halves.
//!
//!   cargo run --example power

fn main() {
    println!("schedule: {:?}", engine::power::schedule(3600, false));
    println!("cancel:   {:?}", engine::power::cancel());
    println!("cancel again (nothing pending, should fail): {}", engine::power::cancel().is_err());
}
