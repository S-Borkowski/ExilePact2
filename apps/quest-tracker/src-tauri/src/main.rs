//! `Quest Tracker.exe` (released as `quest-tracker.exe`): the overlay's entry point.

// No console window behind the overlay in a release build.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    quest_tracker_lib::run()
}
