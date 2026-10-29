"""Developer-only data tooling for the crafting helper.

`poe2db` fetches poe2db.tw's item-class modifier pages into a local cache and
writes the app's checked data files, `apps/crafting-helper/data/index.json`
and one `classes/<class>.json` per item class, from them. Nothing here ships
to players, and nothing here runs inside the app; the app itself (Rust crates,
Tauri shell and the Svelte UI) lives in `apps/crafting-helper/`.
"""
