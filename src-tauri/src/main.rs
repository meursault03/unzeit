#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
fn main() {
    if let Some(code) = unzeit_lib::inference::child_entry() {
        std::process::exit(code);
    }
    unzeit_lib::run();
}
