//! Overlay shell logic without Tauri: app state behind one lock, the worker
//! thread that drains the tailer into the engine and announces a fresh view,
//! the IPC command handlers, mode and click-through state, debounced saves,
//! command-line options and log path discovery.
//!
//! Scaffold stub; the overlay item fills it in.
