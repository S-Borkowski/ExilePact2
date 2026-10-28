# Serialised app-settings views for the Settings window

One file per `update.state` (`idle`, `checking`, `up-to-date`, `available`,
`no-release`, `failed`), each an `AppSettingsView` exactly as `qt-app`
serialises it and the shell sends it as `qt:app-settings`, and nothing else:
`app_settings_fixtures_are_current` fails on any other JSON file here.

`install/` holds one more per `install.state` (`none`, `unsigned`, `ready`,
`downloading`, `verifying`, `restarting`, `failed`), and nothing else, kept
apart so the files at the top stay one per update state.

The views are made by `crates/qt-app/tests/updates.rs`
(`app_settings_fixtures_are_current`) by driving the app through a run with
a fake transport: a fixed synthetic running version, 1.2.0, never the
crate's, so a release bump does not stale them, and 1.3.0 as the newer
release, signed (its exe and `.minisig` listed), so `available` and `failed`
carry the install button. `failed` comes after `available`, so it carries the
newer version a failed check keeps. The install states come from a second
run: `unsigned` from a release without the two assets, `downloading` held
before the exe's first byte (so its progress is exact), `failed` from a
download answered 404, `verifying` the view sent between the download and
the signature check, and `restarting` from an install of the signed test
payload (`fixtures/updates/signing/`). The test fails when a file differs
from what the Rust types serialise; after a deliberate change, regenerate
them and review the diff:

```sh
QT_UPDATE_FIXTURES=1 cargo test --manifest-path apps/quest-tracker/Cargo.toml -p qt-app --locked --test updates
```

The Settings window's UI tests (`src/settings/*.test.ts`) read the files at
the top, which keeps the TypeScript app-settings view in step with the Rust
one, and so does its browser preview (`settings.html?state=<state>`).
