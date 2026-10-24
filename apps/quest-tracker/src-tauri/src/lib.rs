//! Tauri glue for the quest-tracker overlay: the `overlay` window, the
//! commands as thin wrappers over `qt-app`, and the emitter that sends
//! `qt:view` and `qt:toast`.
//!
//! Scaffold stub: it opens the window from `tauri.conf.json` and nothing
//! else; the overlay item fills it in.

/// Builds and runs the Tauri application.
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("quest-tracker: the Tauri application failed to start");
}
