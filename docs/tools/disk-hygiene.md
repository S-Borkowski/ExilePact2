# Disk hygiene: build output, worktrees and free space

What this repository's build output costs on disk, the tool that reports and
reclaims it (`tools/disk_hygiene.py`), why Cargo's output stays in each
checkout, and the numbers measured before and after the dev-profile change.

On 2026-10-30 build outputs filled C: during a workorder (issue #38): every
checkout, the main one and each `git worktree`, builds its apps into its own
`apps/<app>/target/`, a debug `target/` grows by gigabytes per build, and a
`cargo clean` that day freed about 47 GiB. Nothing deletes on its own. A person
runs the tool: on demand, after a pull request merges, and whenever
`/workorder` refuses to launch with `DISK-LOW`.

## Commands

```
py -3 tools/disk_hygiene.py [--repo DIR] report    [--into REF]
py -3 tools/disk_hygiene.py [--repo DIR] worktrees [--into REF] [--apply]
py -3 tools/disk_hygiene.py [--repo DIR] trim      [--cap-gib N] [--apply]
py -3 tools/disk_hygiene.py [--repo DIR] check     [--min-free-gib N]
```

`--repo` is any directory in the checkout (default: the current one), and
`--into` defaults to `origin/main`. The tool is stdlib-only Python. `worktrees`
and `trim` are dry runs unless `--apply`: the dry run prints exactly the lines
`--apply` would act on, then `dry run: nothing changed`, and changes nothing.

| exit | meaning |
|---|---|
| 0 | success (a dry run, a clean apply, `check` at or above its threshold) |
| 1 | an applied action or deletion failed, or `check` is below its threshold |
| 2 | usage: `--repo` is not in a git repository, or `--into` names an unknown ref |

**`report`** lists every registered worktree, the main one first, with its
branch or HEAD and its flags (`main`, `current`, `locked`, `folder missing`,
`dirty`, `status error` (git status failed there), `merged into <ref>` or `not merged into <ref>`), the size of each `apps/*/target` and
`apps/*/node_modules` in it and their total; then each folder under
`<main>/.claude/worktrees/` that git does not list, with its size; then the
free GiB on the drive. It changes nothing.

**`worktrees`** prints one line per registered worktree, starting with its
action word:

| action | when | what `--apply` does |
|---|---|---|
| `keep <path> (<reason>)` | the main worktree; the checkout the tool runs from; locked with its folder present; modified, staged or untracked (not ignored) files; HEAD not an ancestor of `--into`; any git error | nothing |
| `remove <path>` | folder present, unlocked, clean apart from ignored files, HEAD an ancestor of `--into` | `git worktree remove`, without `--force`; if git refuses, the refusal is printed and counted, not overridden |
| `prune <path>` | registered, folder gone, unlocked | `git worktree prune` |
| `prune-locked <path>` | registered, folder gone, locked, and the folder's parent directory exists | `git worktree unlock` on that entry, then `git worktree prune` |
| `unregistered <path>` | a folder under `<main>/.claude/worktrees/` that git does not list | nothing: reported with its size only |

The `remove` line is what reclaims a merged worktree's build output, because
the output lives inside its folder. Branches are never deleted.

**`trim`** checks each present checkout's `apps/*/target` against
`--cap-gib` (default 15 GiB). Under the cap it prints `ok` and leaves it
alone. Over the cap, the `trim` lines name the `<profile>/incremental`
folders first, and the whole `target` only if it would still be over the cap
without them. While deleting it holds that checkout's `build` lock
(`tools/workorder_lock.py`, the lock items already build under) and prints
`busy <checkout>` and skips the checkout when a build holds it. A deletion
error is printed and counted (exit 1), not raised.

**`check`** prints `ok` and exits 0 when the checkout's drive has at least
`--min-free-gib` GiB free (default 30, `DEFAULT_MIN_FREE_GIB`); otherwise it
prints `low`, names `report`, `worktrees` and `trim`, and exits 1.

**The launch check.** `tools/workorder_brief.py --launch-args <slug>
[--min-free-gib N]`, which `/workorder` runs before every launch, makes the
same measurement with the same default. Below the threshold it exits 6 with
nothing on stdout and a stderr line starting `DISK-LOW:` that gives the free
and required GiB and the `report` command. The driver then does not launch:
it runs `report` and the `worktrees` dry run, shows the owner both, and runs
`--apply`, or relaunches with `--min-free-gib 0`, only on the owner's word.
30 GiB because one fresh worktree's three apps took about 13 GiB on
2026-10-30, so 30 leaves room for two.

## What it never removes

- **The main worktree, or the checkout the tool runs from.** Both are always
  `keep`.
- **A worktree with work in it.** Modified, staged or untracked files make it
  `keep`; only files git ignores (build output, `node_modules`) may be
  present for `remove`. `remove` never passes `--force`, so git's own checks
  still apply on top. A worktree that carries a submodule is therefore always
  refused: git will not remove one without `--force`, the tool counts the
  refusal (exit 1), and a person checks it is merged and clean and runs
  `git worktree remove --force <path>` by hand.
- **A worktree whose work is not on `--into`.** Its HEAD has to be an ancestor
  of the ref, `origin/main` by default; run `git fetch` first if the ref is
  stale, because a stale ref only makes the tool keep more.
- **A locked worktree whose folder exists.** The lock is respected; unlock it
  yourself if you mean to remove it.
- **A locked worktree whose folder and its parent folder are both gone.**
  git's lock exists for worktrees on a drive that is not mounted right now,
  and a missing parent is what an unplugged drive looks like, so that entry
  stays `keep`. Only when the parent exists, so the drive is mounted and the
  folder is really gone, does it become `prune-locked`. That case is real:
  on 2026-10-30 four of six removed worktrees stayed registered and locked
  with their folders gone, and `git worktree prune` would not drop them until
  each was unlocked by hand.
- **Anything it found by following a link.** Sizing and deleting never follow
  a symlink or a junction, so a `target` linked elsewhere is neither counted
  nor emptied.
- **Unregistered folders under `.claude/worktrees/`.** They are reported with
  their size and left for a person to delete.
- **Branches, local or remote.** Not by any command.
- **Anything, without `--apply`.** And nothing runs the tool automatically: no
  scheduled task, session hook or workflow step calls it.

## Why build output stays in each checkout

Cargo's output stays at `apps/<app>/target/` in each checkout, as before, and
no `CARGO_TARGET_DIR`, `build.target-dir` or `build.build-dir` is set, tracked
or machine-wide. The disk is reclaimed by removing merged worktrees and
trimming oversized targets instead, and the dev profile now builds
dependencies with `[profile.dev.package."*"] debug = "line-tables-only"`.

That profile table, in the root `Cargo.toml` of each workspace under `apps/`,
gives every non-member
crate (registry crates, and path crates outside the workspace such as the
`apps/shared` crates) line tables only, while the app's own crates keep full
debuginfo. Panic backtraces keep their file and line; the release profile is
untouched. It took the quest tracker's test-build `target` from 4.2 to 3.7 GiB
and its cold build from 127.7 to 99.8 s (the `target` and cold-build rows
below).

The alternatives, and why each was not adopted:

- **One shared `build-dir` for every checkout** (`build.build-dir`, stable on
  cargo 1.98.1; it moves `deps/`, `incremental/` and `.fingerprint/` but keeps
  the final binaries in `target/`). It would reuse registry crates: a second
  checkout pointed at a filled build dir built in 40.6 s against 127.7 s cold.
  But it still rebuilt every workspace and path crate, because their source
  path differs, and that added 1.7 GiB to the shared dir (the "second checkout
  sharing one build dir" row). And Cargo holds one lock on a build dir for the
  whole build: two builds against one dir ran one after the other, 15.9 s for
  the second against 8.5 s alone (the "two builds sharing one build dir" row),
  so a shared dir would serialise every build on the machine, including items
  and workorders in different checkouts.
- **A per-checkout `build-dir` outside the tree** (`{workspace-path-hash}`).
  No lock contention, but no sharing either, and the output would outlive its
  worktree: removing a worktree no longer frees its build output, and a hashed
  folder cannot be traced back to a checkout.
- **A shared `target-dir`.** All of the above, and it would also move every
  `apps/<app>/target/release/<exe>` path that the release workflows, release
  tests, `README.md`, the app guides and the `vite.config.ts` watch ignores
  name. A tracked `.cargo/config.toml` would change CI as well, whose
  `Swatinem/rust-cache` caches `target/`.
- **sccache.** It speeds a cold build but does not shrink `target/`, adds a
  cache of its own (10 GiB by default), does not cache incremental crates and
  needs a download.
- **cargo-sweep.** It needs a `cargo install`; `trim` covers the same need
  with a size cap.
- **pnpm.** `node_modules` is about 0.13 GiB per app, against several GiB of
  Cargo output, and the npm cache is already shared between checkouts.
  Switching would touch three lockfiles, the CI `cache: npm` steps, the
  release tests and every guide's commands.

Each of these can be revisited without touching a tracked path: a later
change can still set `build.build-dir` machine-wide.

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
