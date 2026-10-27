"""Developer-only data tooling for the quest tracker.

This package holds the Python that rebuilds the tracker's quest data after a
game patch: `poe2db` fetches the campaign quest pages from poe2db.tw into a
local cache, drafts a JSON file from them, and measures how far their dialogue
matches a saved `Client.txt`. Nothing here ships to players, and nothing here
runs inside the overlay. The app itself (Rust crates, Tauri shell and the
Svelte UI) lives in `apps/quest-tracker/`.
"""
