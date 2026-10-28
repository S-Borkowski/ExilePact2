# Serialised app-settings views for the Settings window

One file per check state (`update.state`: `idle`, `checking`, `up-to-date`,
`available`, `no-release`, `failed`) and per install state (`install.state`,
the files named `install-<state>`: `install-unsigned`,
`install-downloading`, `install-verifying`, `install-restarting`,
`install-failed`), each an `AppSettingsView` exactly as `wr-app` serialises
it (`ep-update`'s type) and the shell sends it as `wr:app-settings`, and
nothing else: `app_settings_fixtures_are_current` fails on any other JSON
file here. The install state `none` is `idle.json`'s, and `ready` is
`available.json`'s.

The views are made by `crates/wr-app/tests/updates.rs`
(`app_settings_fixtures_are_current`) by driving the app through runs with a
fake transport and the shared signature fixtures
(`apps/shared/fixtures/signing/`): a fixed synthetic running version, 1.2.0,
never the crate's, so a release bump does not stale them, and 1.3.0 as the
newer release. `failed` comes after `available`, so it carries the newer
version a failed check keeps; `install-failed` is a signature by another
key. No view carries a URL. The test fails when a file differs from what the
Rust types serialise; after a deliberate change, regenerate them and review
the diff:

```sh
WR_UPDATE_FIXTURES=1 cargo test --manifest-path apps/waystone-roller/Cargo.toml -p wr-app --locked --test updates
```

The Settings window's UI tests (`src/settings/*.test.ts`) read them, which
keeps the TypeScript app-settings view in step with the Rust one, and so
does its browser preview (`settings.html?state=<name>`).
