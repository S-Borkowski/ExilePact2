//! `waystone-roller.exe`: the window's entry point.

// No console window behind the app in a release build.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    waystone_roller_lib::run()
}
