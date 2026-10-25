# Review of the row transition

The design check of the row's own transition, which replaced the toast stack
after Live 1 (owner, 2026-10-25): a `qt:row-change` plays on the quest row it
names (`apps/quest-tracker/src/components/QuestRow.svelte`), is announced in
a polite live region (`apps/quest-tracker/src/App.svelte`), and the header
labels the provisional character "New character"
(`apps/quest-tracker/src/components/Header.svelte`). The spec is
`component-spec.md` § "Row transition" and the brief's frame 9. Tools:
impeccable 4.7.1 (`impeccable detect`, then its `audit` checklist, applied
by the agent that built the change), and the `animate` skill for the motion
choices. The skill ran as a subagent with no owner to ask: its interview step
was replaced by the brief, the component spec and the Decisions entry of
2026-10-25.

audit: ran (source only). The detector found nothing over `QuestRow.svelte`, `App.svelte` and `Header.svelte` (`detect --json` printed `[]`). Checklist, 19/20: accessibility 4 (each change is read once from a polite region, a repeated sentence is a new node and so is read again, the step leaving is `aria-hidden`, reduced motion keeps the state change and the announcement and drops only the movement, play mode stays free of controls); performance 4 (opacity and transform only, on one line per row, about 370 ms in all, no `will-change`, no layout property animated); theming 3 (the durations come from `rowchange.ts` into the CSS through `--row-out-ms` and `--row-in-ms`; the curve is a local `--row-ease`, because the harvest designed no `motion.easing`); responsive 4 (no new target, no width); integrity 4. The rendered check (`detect --cdp` against the running WebView2) did not run: this item builds no app, so the motion was checked by the timed tests in `src/row-change.test.ts` and not by eye.
findings: none blocking; nothing to fix in this item's files. Two notes, decided rather than fixed:

- **N1** — a quest done while its row showed a step: the view (sent first)
  already draws the compact finished row, and the step that was completed
  then fades out inside it, so that row is a line taller for 150 ms and then
  settles. That is the spec's "the row settles into its finished variant".
  Pinning the old height would mean keeping the in-progress layout after the
  view says done, which the UI does not do (Rust decides, the UI renders).
- **N2** — `prefers-reduced-motion: reduce` makes the change instant, with
  no fade, which is the owner's default (Decisions, 2026-10-25). impeccable
  and the `animate` skill would keep a short opacity fade there. To undo,
  keep the `out` and `in` phases under reduced motion and drop only the
  `translateY` in `QuestRow.svelte`'s reduced-motion block.

Motion, as built: the completed step fades to 0 over 150 ms, then the next
step (or, when the row has none, as for a quest just available, the row's
title) comes in from 0.2 opacity, rising 5 px, over 220 ms, on
`cubic-bezier(0.23, 1, 0.32, 1)`, the `animate` skill's strong ease-out for
entering and leaving. A second change re-keys the row's content and clears
the first change's timers, so it starts again from its own beginning and
nothing queues. A row that was not on screen when its change came (its act
folded, filtered out, the panel collapsed, or the detail open) is announced
only. When it shows up later it is drawn as it is now, and the old change is
not played.

What to check by eye at the next live run: the 150 + 220 ms rhythm at 1x
and 2x, over a bright scene and a dark one.
