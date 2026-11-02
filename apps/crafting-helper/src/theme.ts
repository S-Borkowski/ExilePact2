// The theme every page draws (`ui-design.md` § "Theme"): Exile, the quest
// tracker's steel/masonry kit, by default, or Claude, the app's earlier
// look. The view chooses it (`theme` in `View` and `SettingsView`); until
// the first view arrives, and for a view without one, it is Exile. The
// choice is one attribute, `data-theme` on the document element: every
// Exile rule (`../theme/exile.css`, `steel.css`) is scoped to
// `:root[data-theme='exile']`, so Claude draws today's rules untouched.

import { THEMES, type Theme } from './ipc';

/** Marks the document with the view's theme (Exile when it names none). */
export function applyTheme(root: HTMLElement, theme: Theme | undefined): void {
  root.dataset.theme = theme === 'claude' ? 'claude' : 'exile';
}

/** The theme a preview's query asks for (`theme=exile|claude`), or null for none or an unknown one. */
export function previewTheme(search: string): Theme | null {
  const theme = new URLSearchParams(search).get('theme');
  return theme && (THEMES as readonly string[]).includes(theme) ? (theme as Theme) : null;
}
