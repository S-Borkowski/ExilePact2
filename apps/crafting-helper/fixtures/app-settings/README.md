# Serialised Settings window views

One file per check state (`update.state`: `idle`, `checking`, `up-to-date`,
`available`, `no-release`, `failed`) and per install state (`install.state`,
the files named `install-<state>`: `install-unsigned`,
`install-downloading`, `install-verifying`, `install-restarting`,
`install-failed`), then `hotkey-refused` (`set_hotkey` asked for the game's
own Ctrl+Alt+C) and `blocked-currencies` (a list of three catalogue ids). Each is a
`SettingsView` exactly as `ch-app` serialises it and the shell sends it as
`ch:app-settings`: `ep-update`'s `AppSettingsView` fields (`version`,
`check_updates`, `update`, `install`) with `hotkey`, `hotkey_status` and
`blocked_currencies` beside them. Nothing else may be here:
`fixtures_are_current` fails on any other JSON file. The install state
`none` is `idle.json`'s, and `ready` is `available.json`'s.

The views are made by `crates/ch-app/tests/fixtures.rs`
(`fixtures_are_current`) by driving the app through runs with a fake
transport and the shared signature fixtures (`apps/shared/fixtures/signing/`):
a fixed synthetic running version, 1.2.0, never the crate's, so a release
bump does not stale them, and 1.3.0 as the newer release. `failed` comes
after `available`, so it carries the newer version a failed check keeps;
`install-failed` is a signature by another key. No view carries a URL. The
test fails when a file differs from what the Rust types serialise; after a
deliberate change, regenerate them and review the diff:

```sh
CH_FIXTURES=1 cargo test --manifest-path apps/crafting-helper/Cargo.toml -p ch-app --locked --test fixtures
```

The Settings window's UI tests read them, which keeps the TypeScript view in
step with the Rust one.
