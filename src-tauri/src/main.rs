//! Binary entry point. Everything lives in `serialclon_lib` so the same code
//! can be linked into tests and, later, a CLI.

// Prevents an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    serialclon_lib::run()
}
