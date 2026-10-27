// A live change of a quest's row (`qt:row-change`), in place of the toasts
// (owner, 2026-10-25, after Live 1). The row whose `id` is the change's
// `quest_id` plays it: the completed step fades out, then the step now
// current fades in. A second change to the same row restarts it; nothing
// queues. With `prefers-reduced-motion: reduce` the change is instant. Every
// change is announced in a polite live region, shown row or not.
//
// Whatever the motion setting, the row (or, in the `collapsed` frame, the
// badge) is also highlighted: tinted at once, held `ROW_HIGHLIGHT_MS`, then
// cleared, and a second change restarts the hold (owner, after Live 2:
// Windows' animations were off and nothing marked the change). The tint is
// colour only, so it shows under reduced motion too.

import { getContext, setContext } from 'svelte';
import { SvelteMap } from 'svelte/reactivity';
import type { RowChange } from './ipc';

/** The completed step fading out. */
export const ROW_STEP_OUT_MS = 150;
/** The next step fading in, from 0.2 opacity, moving up 5 px. */
export const ROW_STEP_IN_MS = 220;
/** How long a changed row or the badge stays highlighted; a second change restarts it. */
export const ROW_HIGHLIGHT_MS = 2500;
/** The highlight's fade back out (it comes on at once). */
export const ROW_HIGHLIGHT_OUT_MS = 220;
/** How many announcements the live region keeps in the DOM (it reads additions only). */
export const ANNOUNCED_KEEP = 4;

/** Where a row is in its transition: `out` shows the completed step leaving, `in` the new state arriving. */
export type RowPhase = 'idle' | 'out' | 'in';

/** Whether the system asks for reduced motion (Windows: animations off). */
export function prefersReducedMotion(): boolean {
  return typeof window !== 'undefined' && typeof window.matchMedia === 'function'
    ? window.matchMedia('(prefers-reduced-motion: reduce)').matches
    : false;
}

/**
 * The latest change per quest, shared by the root and every row. Keyed by
 * quest, so two changes for two rows in one batch both play; a row reads
 * its own key only.
 */
export class RowChanges {
  readonly byQuest = new SvelteMap<string, RowChange>();
  /** The highest change id seen; a row mounted later ignores every change up to it. */
  latest = 0;

  add(change: RowChange): void {
    this.byQuest.set(change.quest_id, change);
    if (change.id > this.latest) this.latest = change.id;
  }

  /** The newest change of any quest, read reactively (the badge plays every one). */
  newest(): RowChange | null {
    let found: RowChange | null = null;
    for (const change of this.byQuest.values()) if (!found || change.id > found.id) found = change;
    return found;
  }
}

const KEY = Symbol('quest-tracker row changes');

export function setRowChanges(changes: RowChanges): RowChanges {
  return setContext(KEY, changes);
}

/** The root's changes, or none (a row mounted on its own, as in a test). */
export function getRowChanges(): RowChanges | null {
  return getContext<RowChanges | undefined>(KEY) ?? null;
}
