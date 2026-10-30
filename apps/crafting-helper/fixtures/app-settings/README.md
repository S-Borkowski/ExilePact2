# Serialised Settings window views

One file per check state (`update.state`: `idle`, `checking`, `up-to-date`,
`available`, `no-release`, `failed`) and per install state (`install.state`,
the files named `install-<state>`: `install-unsigned`,
`install-downloading`, `install-verifying`, `install-restarting`,
`install-failed`), then `hotkey-refused` (`set_hotkey` asked for the game's
own Ctrl+Alt+C), `blocked-currencies` (a list of three catalogue ids) and
`overlay-settings` (the overlay at `bottom-right`, not the default
`top-center`, and the screen watch off; every other view has the defaults),
then the Trade section's three: `trade-league` (`Runes of Aldur` chosen,
not the default `Forbidden Rites`, the shipped list offered, no note),
`trade-league-refreshing` (`refresh_trade_leagues` answered at once while
the fake shell holds the league request in flight: refreshing, the list as
it was) and `trade-league-refresh-refused` (the site answered 503: the
note says so, the shipped list and the league kept). Every other view has
the default league, the shipped list, no refresh running and no note.
Each is a `SettingsView` exactly as `ch-app` serialises it and the shell
sends it as `ch:app-settings`: `ep-update`'s `AppSettingsView` fields
(`version`, `check_updates`, `update`, `install`) with `hotkey`,
`hotkey_status`, `blocked_currencies`, `overlay_position`,
`screen_watch`, `trade_league`, `trade_leagues`,
`trade_leagues_refreshing` and `trade_league_note` beside them. Nothing
else may be here:
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
