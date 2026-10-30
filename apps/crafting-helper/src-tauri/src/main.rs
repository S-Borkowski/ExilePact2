//! `Crafting Helper.exe`: the window's entry point. Cargo builds it as
//! `crafting-helper.exe`; `tauri build` renames it from `mainBinaryName`.

// No console window behind the app in a release build.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    crafting_helper_lib::run()
}
