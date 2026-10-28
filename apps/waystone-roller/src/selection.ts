// What a row's three-way choice sends, and the small facts the page reads
// off a view. Nothing here builds or measures the search: that is the
// shell's.

import type { ModView, Selection, View } from './ipc';
import type { SelectionPatch } from './bridge';

export type Choice = 'want' | 'none' | 'avoid';

/** A mod's current choice in the selection. */
export function choiceOf(selection: Selection, id: string): Choice {
  if (selection.want.includes(id)) return 'want';
  if (selection.avoid.includes(id)) return 'avoid';
  return 'none';
}

/**
 * The patch that gives `id` the choice `to`: both lists, the mod moved out of
 * the other one, so the shell never sees a mod in both. Order is kept, and a
 * newly chosen mod goes last.
 */
export function choicePatch(selection: Selection, id: string, to: Choice): SelectionPatch {
  const want = selection.want.filter((m) => m !== id);
  const avoid = selection.avoid.filter((m) => m !== id);
  if (to === 'want') want.push(id);
  if (to === 'avoid') avoid.push(id);
  return { want, avoid };
}

/** The page's three groups, in order. */
export const GROUPS = [
  { id: 'prefixes', title: 'Prefixes' },
  { id: 'suffixes', title: 'Suffixes' },
  { id: 'desecrated', title: 'Desecrated' },
] as const;
export type GroupId = (typeof GROUPS)[number]['id'];

export function groupOf(mod: ModView): GroupId {
  if (mod.group === 'desecrated') return 'desecrated';
  return mod.kind === 'prefix' ? 'prefixes' : 'suffixes';
}

/** Whether a mod matches the list's text filter (its lines, affix name and token; case-insensitive). */
export function matchesFilter(mod: ModView, filter: string): boolean {
  const needle = filter.trim().toLowerCase();
  if (needle === '') return true;
  return [...mod.lines, mod.name ?? '', mod.token ?? ''].some((text) => text.toLowerCase().includes(needle));
}

/** The source line's site name: the data URL's host without `www.`. */
export function sourceSite(view: View): string {
  try {
    return new URL(view.data.source).hostname.replace(/^www\./, '');
  } catch {
    return view.data.source;
  }
}

export const plural = (n: number, one: string, many: string): string => `${n} ${n === 1 ? one : many}`;
