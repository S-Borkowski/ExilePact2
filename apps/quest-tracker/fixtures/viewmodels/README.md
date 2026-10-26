# Serialised views for the UI tests and the browser preview

One file per frame id of the brief's `## Export contract`
(`docs/quest-tracker/ui-design-brief.md`), each a `View` exactly as
`qt-viewmodel` serialises it and the shell sends it as `qt:view`, and
nothing else: `fixtures_match_serialised_views` fails on any other JSON file
here. `mode-play` and `mode-edit` have no file: they are the same panel with
`mode` set to `play` or `edit`. The toasts and `toast.json` are gone (owner,
2026-10-25, after Live 1): a live change is a `RowChange` sent as
`qt:row-change`, and the row the frames' story ends on is built in the test
(`story_row_change`), not stored.

The views are built by `crates/qt-viewmodel/tests/viewmodel.rs`
(`fixtures_match_serialised_views`) from the engine's synthetic data
(`../quests_min.json`) and one session written for them,
`../sessions/frames/story.txt` (its `README.md` tells the story), so every
name in them is invented. The test fails when a file differs from what the
Rust types serialise; after a deliberate change to the types or the rules,
regenerate them and review the diff:

```sh
QT_UPDATE_FIXTURES=1 cargo test --manifest-path apps/quest-tracker/Cargo.toml -p qt-viewmodel --locked
```

Two files read these views: the UI tests (`src/*.test.ts`), which is what
keeps the TypeScript types in step with the Rust ones, and the browser
preview (`npm --prefix apps/quest-tracker run dev`, then
`http://127.0.0.1:1420/?frame=<frame id>`, `src/preview.ts`), which draws
any frame without the game.

## One story

Every frame that shows a character shows the same moment of one story
(finish review F12), and two tests hold the set to it:

- `fixture_frames_share_one_character_state`: one active character,
  `Corisande_Fell` (Witch, level 5), and each listed character at one class
  and level in every frame (the picker's second character, `Odile_Marsh`,
  level 3, is listed everywhere); each quest has one status, worth and next
  step in every frame but `empty-act-done`, whose scenario marks the rest of
  the act done; each act's counts follow its rows (below); and the row
  change the story ends on (Una's line read live: the lute becomes
  available) announces a status the frames show.
- `fixture_rows_cover_every_worth`: across the set the rows carry every
  worth (`yes`, `optional`, `no`), every detection (`auto`, `partial`,
  `manual`) and both auto and manual-tick rows; `panel-next-up` alone shows
  both kinds of row and more than one worth.

The moment: act 1 holds one quest in each status. Riverbank done, the pack
leader (`fixture-beira`) skipped and worth `no`, the bell locked, the king
in progress and worth `optional`, the lute available (partial, a
manual-tick row), the candles unknown (manual). The riverbank is done by
inference (its data carries `done_inferred`), so its row carries the
"probably done" mark (`inferred: true`), and the king's first step, which
leaves no trace in the log, shows done by inference once Freythorn was
entered. The king's current step, "Face the king", carries a find-it tip
(`current_tip` on its rows, `tip` on its step in the detail). The two worth
values and the tip are changed on the test's own copy of the data; `../quests_min.json` and the
sessions beside `frames/` stay as the engine, replay and shell tests read
them.

## The act count rule

An act's header reads `done/total`, where `done` counts the act's done rows
and `total` every row but the skipped ones (`crates/qt-viewmodel/src/lib.rs`,
the act view's `total:` line). With "show skipped" on, a skipped row is
listed but not counted: `panel-campaign.json` lists six act-1 rows and reads
`1/5`. That is the rule, not a fixture error. With "only worth it" on, the
counts still cover the whole act, so they can exceed the rows shown.

| File | Built from the story, then |
|---|---|
| `panel-campaign.json` | campaign view, "show skipped" on: one act-1 row in each status, `1/5`; the done row inferred; next up also lists the skipped pack leader (available by its tracked progress) after the unskipped rows |
| `panel-next-up.json` | next up: the lute (worth `yes`, available) before the king (`optional`, in progress, its next step and that step's tip); the preview's default frame |
| `collapsed.json` | the panel folded: the badge counts the one worth-it quest available |
| `quest-detail.json` | edit mode, the king's detail: its first step done by inference, the second done, the third current, with its tip |
| `character-picker.json` | edit mode, the picker: Corisande active, Odile the second own character |
| `settings.json` | edit mode, a log path set |
| `empty-no-log.json` | nothing read, no log found: an error status |
| `empty-waiting.json` | nothing read, the log there |
| `empty-no-character.json` | `../sessions/baseline.txt`: lines, but no character |
| `empty-act-done.json` | the rest of act 1 marked done (the pack leader stays skipped): `5/5` |