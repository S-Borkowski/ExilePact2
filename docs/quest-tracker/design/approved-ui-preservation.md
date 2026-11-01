# Approved UI feature preservation

The order matches the 22 existing feature groups in [the design system](../../ui-design-system.md#quest-tracker-feature-preservation). Exile changes presentation; Claude retains the prior component CSS, fonts and palette. Rust still owns state. Theme is the sole added preference, in both Settings surfaces, saved as `settings.theme` (`exile` by default, or `claude`).

| # | Existing feature | Implementation | Verification |
|---|---|---|---|
| 1 | Passive play / interactive edit; close edit frames on return | Existing shell modes and shared App/Panel behavior; selected material remains unchanged across modes | app.test.ts; finish-frames.test.ts; approved-ui.test.ts; Rust shell |
| 2 | Global shortcut and binding/refusal states | Existing Settings hotkey and edit footer | live2.test.ts; Rust shell |
| 3 | Header drag, eight resize directions, saved bounds, independent zoom | Existing bridge/native geometry; new-store364×384, minimum320×240 | resize.test.ts; Rust shell saved-size/theme tests |
| 4 | Collapse/expand, count, warning and change badge | Existing Collapsed with selected theme tokens | app.test.ts; live2.test.ts; row-change.test.ts |
| 5 | Next up / All, worth grouping, act disclosures | Existing Header, Panel and ActSection commands | app.test.ts; finish-header.test.ts; finish-rows.test.ts |
| 6 | Statuses, worth, rewards, step/tip, manual/inferred marks | Same QuestRow data; Exile wrapping and original Claude layout | finish-rows.test.ts; live3.test.ts; inferred.test.ts |
| 7 | Detail facts, reason, steps/tips, rewards and fetched date | Same in-panel QuestDetail | finish-detail.test.ts; app.test.ts |
| 8 | Tick/untick, inferred refusal, done/unmark, skip/unskip, confirmed reset | Same detail actions and timers | finish-detail.test.ts; inferred.test.ts; Rust shell |
| 9 | Skipped progress and retained manual marks | Existing engine/store and Show skipped preference | live3.test.ts; Rust viewmodel/shell |
| 10 | Open source and fetched date | Existing open_source payload and metadata | app.test.ts; finish-detail.test.ts |
| 11 | Character detection/name/class/level/new/unconfirmed, select/pin/mine | Existing Header and CharacterPicker | finish-header.test.ts; finish-frames.test.ts; Rust shell |
| 12 | Show skipped and Only worth-it | Existing settings patches and Rust filtering | app.test.ts; Rust viewmodel |
| 13 | Opacity0–100%, Auto/fixed zoom and live sliders | Same slider mechanics; theme never resizes window | live2.test.ts; live3.test.ts; resize.test.ts; Rust shell |
| 14 | Opt-in screen reading/privacy/refusals and automatic/override log | Existing controls, helpers and source settings | screen-reading.test.ts; app.test.ts |
| 15 | Automatic detection, per-character persistence and catch-up | Rust detection/store unchanged | Rust shell/viewmodel; row-change.test.ts |
| 16 | Game-only/pre-area play visibility, edit visibility, panels/Escape exception | Native visibility logic unchanged | hide-under-panels.test.ts; Rust shell |
| 17 | Row/step highlight, reduced motion, one announcement, no catch-up animation | Existing rowchange/Collapsed timers and live region | row-change.test.ts; live2.test.ts |
| 18 | Warnings/errors and all empty states/recovery | Shared Panel/EmptyState and selected material shell | app.test.ts; finish-frames.test.ts; rendered review |
| 19 | Ordinary Settings automatic/manual check/status/version/credit | Separate SettingsWindow retained; Appearance alongside Updates | settings-window.test.ts; Rust updates |
| 20 | Signed install/restart/progress/cancel/retry/refusal | Updater commands/fields unchanged; QT-local view adds theme | settings-window.test.ts; Rust updates |
| 21 | Tray entries and hidden-overlay quit | Existing tray/action IDs unchanged | Rust updates/shell |
| 22 | Keyboard/focus/shapes/announcements and runtime/developer interfaces | Shared controls and existing commands; update_settings.theme added | ipc.test.ts; approved-ui.test.ts; rendered review; Rust shell |

Theme checks cover default/invalid stored values, atomic refusal of invalid patches, both windows receiving the choice, no updater request, saved preference reload, unchanged geometry, local licensed assets and separate JSON/CSS value parity. Every expanded frame retains its selected material across play/edit; mode changes request no theme patch and play remains control-free. See [rendered review](review-approved-ui.md). No game integration mechanism is introduced by this restyle.
