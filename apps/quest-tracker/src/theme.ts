// The theme: every token key has a default in `defaults.css`; the harvested
// `theme/tokens.css`, `theme/theme.json` and `theme/icons/<id>.svg` override
// them when present. All are reached through `import.meta.glob`, which
// matches nothing when the files are absent, so the UI builds and runs with
// or without the harvest.
//
// `main.ts` imports `defaults.css` before this module: Vite hoists a glob's
// imports to the top of its module, so importing the defaults here would put
// them after the tokens. Same `:root` selector, so the later file wins.
import.meta.glob('../theme/tokens.css', { eager: true });
import.meta.glob('../theme/exile.css', { eager: true });

type ThemeValues = Record<string, string | number>;

const THEME_FILES = import.meta.glob('../theme/theme.json', { eager: true, import: 'default' }) as Record<
  string,
  ThemeValues
>;

/** The harvested `theme.json` (token key -> value), or null before the harvest. */
export const THEME: ThemeValues | null = Object.values(THEME_FILES)[0] ?? null;

const EXILE_FILES = import.meta.glob('../theme/exile.json', { eager: true, import: 'default' }) as Record<string, ThemeValues>;
export const EXILE_THEME: ThemeValues | null = Object.values(EXILE_FILES)[0] ?? null;

/** A received view chooses the cascade; no independent browser preference. */
export function applyPresentationTheme(root: HTMLElement, theme: 'exile' | 'claude' = 'exile'): void {
  root.dataset.theme = theme;
  root.style.removeProperty('--drop-panel');
  applyDerivedTokens(root, theme === 'exile' ? EXILE_THEME ?? THEME : THEME);
}

const RAW_ICONS = import.meta.glob('../theme/icons/*.svg', {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>;

/**
 * An icon's SVG with its own accessible name removed (`<title>`, `role`,
 * `aria-labelledby`): the element around it carries the label, and the ids
 * inside a `<title>` would repeat once per row.
 */
function inlineable(svg: string): string {
  return svg
    .replace(/<title\b[^>]*>[\s\S]*?<\/title>/g, '')
    .replace(/\s(?:role|aria-labelledby)="[^"]*"/g, '')
    .replace(/<svg\b/, '<svg aria-hidden="true" focusable="false"');
}

const ICONS: Record<string, string> = Object.fromEntries(
  Object.entries(RAW_ICONS).map(([path, svg]) => [path.replace(/^.*\/([^/]+)\.svg$/, '$1'), inlineable(svg)]),
);

/** The harvested SVG for an icon id, or null when it has none (`icons` is for tests). */
export function iconSvg(id: string, icons: Readonly<Record<string, string>> = ICONS): string | null {
  return icons[id] ?? null;
}

/**
 * A text glyph per icon id, drawn when the harvested SVG is missing. The
 * status glyphs differ in shape, not only in colour (the brief's rule).
 */
export const GLYPHS: Record<string, string> = {
  'status-locked': '■',
  'status-available': '○',
  'status-in-progress': '◐',
  'status-done': '✓',
  'status-skipped': '»',
  'status-unknown': '?',
  'worth-yes': '★',
  'worth-optional': '☆',
  'worth-no': '–',
  'manual-tick': '✎',
  'ui-chevron': '›',
  'ui-collapse': '▾',
  'ui-expand': '▸',
  'ui-settings': '⚙',
  'ui-close': '×',
  'ui-edit': '✎',
  'ui-click-through': '⇱',
  'ui-external-link': '↗',
  'ui-character': '☺',
  'ui-refresh': '↻',
  // The harvest's extras beyond the contract.
  'ui-chevron-down': '⌄',
  'ui-chevron-up': '⌃',
  'ui-chevron-left': '‹',
  'ui-check': '✓',
  'ui-warning': '!',
};

/** The glyph for an icon id; reward icons share a plain diamond. */
export function iconGlyph(id: string): string {
  return GLYPHS[id] ?? (id.startsWith('reward-') ? '◆' : '•');
}

/** The top-level comma-separated parts of a CSS value (commas inside `rgba(...)` stay). */
function splitTopLevel(value: string): string[] {
  const parts: string[] = [];
  let depth = 0;
  let start = 0;
  for (let i = 0; i < value.length; i++) {
    const c = value[i];
    if (c === '(') depth++;
    else if (c === ')') depth--;
    else if (c === ',' && depth === 0) {
      parts.push(value.slice(start, i));
      start = i + 1;
    }
  }
  parts.push(value.slice(start));
  return parts.map((p) => p.trim()).filter(Boolean);
}

/**
 * A `box-shadow` list as a CSS `filter` of `drop-shadow()`s. The panel, the
 * badge have cut corners (`clip-path`), which clips their own
 * `box-shadow`; a drop shadow on a wrapper follows the cut shape instead.
 * `drop-shadow` has no spread, so a layer's spread is added to its blur; inset
 * layers have no outer equivalent and are dropped.
 */
export function dropShadowFilter(boxShadow: string): string {
  return splitTopLevel(boxShadow)
    .filter((layer) => !/\binset\b/.test(layer))
    .map((layer) => {
      const colour = layer.match(/(?:rgba?|hsla?|color-mix)\([^)]*\)|#[0-9a-f]{3,8}\b|\b[a-z]+$/i)?.[0] ?? '';
      const lengths = layer.replace(colour, '').trim().split(/\s+/).filter(Boolean);
      const [x = '0', y = '0', blur = '0px', spread = '0px'] = lengths;
      const px = (v: string) => Number.parseFloat(v) || 0;
      const widened = `${px(blur) + px(spread)}px`;
      return `drop-shadow(${x} ${y} ${widened}${colour ? ` ${colour}` : ''})`;
    })
    .join(' ');
}

/**
 * Sets the custom properties derived from the harvested theme rather than
 * copied from it: the drop-shadow filter of `shadow.panel`. Without the harvest nothing is set and `defaults.css`
 * applies.
 */
export function applyDerivedTokens(root: HTMLElement, theme: ThemeValues | null = THEME): void {
  if (!theme) return;
  const derived: Array<[string, string]> = [
    ['shadow.panel', '--drop-panel'],
  ];
  for (const [key, property] of derived) {
    const value = theme[key];
    if (typeof value === 'string' && value.trim()) root.style.setProperty(property, dropShadowFilter(value));
  }
}
