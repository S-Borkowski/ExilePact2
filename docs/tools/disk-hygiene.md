# Disk hygiene: build output, worktrees and free space

What this repository's build output costs on disk, measured before and after
the dev-profile change, and why Cargo's output stays in each checkout.

## Measurements

Quest tracker only: it is the largest app, so it stands for the others.
`before` was measured on 2026-10-31 at `748230ff` (dependencies built with
full debuginfo), cargo 1.98.1 on Windows 11, with
`npm --prefix apps/quest-tracker run build` run once first because the Tauri
crate embeds `dist/`. Each build ran under
`py -3 tools/workorder_lock.py build`, and only the cargo command was timed.
Sizes are on-disk bytes under the folder, hard links counted once, in GiB.
The build rows were taken twice; the table gives the second run (the first:
117.9 s cold, 14.0 s warm). `after` was measured the same way on the same
day at `24f6e023`, with `[profile.dev.package."*"] debug =
"line-tables-only"` committed (the first run: 107.1 s cold, 7.5 s warm).

| measurement | before | after | how |
|---|---|---|---|
| cold test build, quest-tracker | 127.7 s | 99.8 s | delete `apps/quest-tracker/target`, then `cargo test --manifest-path apps/quest-tracker/Cargo.toml --workspace --locked --no-run` |
| warm test build, quest-tracker | 10.0 s | 6.5 s | touch `apps/quest-tracker/crates/qt-engine/src/lib.rs`, then the same `cargo test ... --no-run` |
| target after the test build, quest-tracker | 4.2 GiB | 3.7 GiB | size of `apps/quest-tracker/target` after the cold and warm builds |
| incremental share of that target | 1.3 GiB | 1.3 GiB | size of every `incremental` folder under that `target` |
| .pdb share of that target | 1.0 GiB | 0.8 GiB | size of every `*.pdb` file under that `target` |
| free space on C: | 121.6 GiB | 117.8 GiB | `shutil.disk_usage("C:/").free`: `before` at the start of the before measurements, `after` at the end of the after ones (115.7 GiB at their start) |
| second checkout sharing one build dir | 40.6 s, and the shared dir grew 1.7 GiB (4.2 to 5.9 GiB); the checkout's own `target` held 0.06 GiB | none: before only | `git worktree add --detach .claude/worktrees/measure-reuse HEAD`, copy `apps/quest-tracker/dist` into it, run the cold command there with `CARGO_BUILD_BUILD_DIR` set to this checkout's `apps/quest-tracker/target` (filled by the builds above), size both; then `git worktree remove --force` |
| two builds sharing one build dir | 15.9 s for the second build, which waited on the build-dir lock, against 8.5 s alone (17 s wall for both) | none: before only | the planner's toy result, not re-measured here: two toy crates with an 8-second build script each, built at once against one fixed `CARGO_BUILD_BUILD_DIR` |
| parallel items launch | not measured: nothing here changes it, because output stays per checkout and items already serialise builds through the `build` lock | | none |

Between the columns only the dev profile changed: dependencies now keep line
tables instead of full debuginfo, which saved 0.5 GiB of the quest tracker's
`target` (4.2 to 3.7 GiB, 0.2 GiB of it in `.pdb` files) and made the cold
build faster (99.8 against 127.7 s, and 107.1 against 117.9 s on the first
runs, so build time is noisier than size); the free-space row moved with everything else on
the machine and is not a measure of the change.

What the shared-dir row says: a second checkout pointed at a filled build
dir reused the registry crates (40.6 s against 127.7 s cold) but still
rebuilt the workspace and path crates, whose source path differs, and that
added 1.7 GiB to the shared dir. The toy row says the price: Cargo holds one
lock on a build dir for the whole build, so a dir shared by every checkout
serialises every build on the machine.
