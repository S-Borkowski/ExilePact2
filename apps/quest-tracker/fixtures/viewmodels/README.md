# Serialised views for the UI tests

One file per frame id of the brief's `## Export contract`
(`docs/quest-tracker/ui-design-brief.md`), each a `View` exactly as
`qt-viewmodel` serialises it and the shell sends it as `qt:view`, plus
`toast.json`, one `Toast` as sent as `qt:toast`. `mode-play` and `mode-edit`
have no file: they are the same panel with `mode` set to `play` or `edit`.

The views are built by `crates/qt-viewmodel/tests/viewmodel.rs`
(`fixtures_match_serialised_views`) from the engine's synthetic data
(`../quests_min.json`) and sessions (`../sessions/`), so every name in them
is invented. The test fails when a file differs from what the Rust types
serialise; after a deliberate change to the types or the rules, regenerate
them and review the diff:

```sh
QT_UPDATE_FIXTURES=1 cargo test --manifest-path apps/quest-tracker/Cargo.toml -p qt-viewmodel --locked
```

The UI tests (`src/*.test.ts`) read these files, which is what keeps the
TypeScript types in step with the Rust ones.

| File | Built from |
|---|---|
| `panel-campaign.json` | `act1.txt` without Beira's line, the reward and the Hunting Grounds, the lute skipped, "show skipped" on: one act-1 row in each status |
| `panel-next-up.json` | `act1.txt`: in-progress quests, then available ones, each with its next step |
| `collapsed.json` | `act1.txt`, panel folded: the badge count |
| `quest-detail.json` | `act1.txt`, edit mode, the king's detail: one step done, the next current |
| `character-picker.json` | `switch.txt`: two own characters, one active |
| `settings.json` | `act1.txt`, edit mode, a log path set |
| `empty-no-log.json` | nothing read, no log found: an error status |
| `empty-waiting.json` | nothing read, the log there |
| `empty-no-character.json` | `baseline.txt`: lines, but no character |
| `empty-act-done.json` | `act1.txt` with the rest of act 1 marked done |
| `toast.json` | `act1.txt` caught up to the Hunting Grounds, which are then entered live |
