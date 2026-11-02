// The look a page draws. `settings.json`'s `theme` reaches both windows in
// their views; each page sets `data-theme` on its root element from the
// view it receives, `exile` until the first arrives (as the Quest Tracker's
// pages do). Every steel rule (`../theme/exile.css`, `./steel.css`) is
// scoped to `:root[data-theme='exile']`, so `claude` draws the page as it
// was before the steel/masonry kit.

import { THEMES, type Theme } from './ipc';

/** The theme a value names: exactly `claude` is Claude; anything else, a missing one included, is Exile. */
export function themeOf(value: unknown): Theme {
  return value === 'claude' ? 'claude' : 'exile';
}

/** Sets the page's look from a view's `theme`. */
export function applyTheme(root: HTMLElement, theme: unknown): void {
  root.dataset.theme = themeOf(theme);
}

/** The browser preview's `theme` parameter (`exile`|`claude`), or null for none or another value. */
export function previewTheme(search: string): Theme | null {
  const theme = new URLSearchParams(search).get('theme');
  return theme && (THEMES as readonly string[]).includes(theme) ? (theme as Theme) : null;
}
