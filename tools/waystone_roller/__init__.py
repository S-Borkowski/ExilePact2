"""Developer-only data tooling for the waystone roller.

`poe2db` fetches poe2db.tw's Waystones page into a local cache and writes the
app's mod data file, `apps/waystone-roller/data/waystone-mods.json`, from it.
Nothing here ships to players, and nothing here runs inside the app; the app
itself (Rust crates, Tauri shell and the Svelte UI) lives in
`apps/waystone-roller/`.
"""
