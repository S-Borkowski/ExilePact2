// The theme: every token key has a default in `defaults.css`; the harvested
// `theme/tokens.css` and `theme/icons/<id>.svg` override them when present.
// Both are reached through `import.meta.glob`, which matches nothing when the
// files are absent, so the UI builds and runs with or without the harvest.
//
// `main.ts` imports `defaults.css` before this module: Vite hoists a glob's
// imports to the top of its module, so importing the defaults here would put
// them after the tokens. Same `:root` selector, so the later file wins.
import.meta.glob('../theme/tokens.css', { eager: true });

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

/** The harvested SVG for an icon id of the brief, or null before the harvest. */
export function iconSvg(id: string): string | null {
  return ICONS[id] ?? null;
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
};

/** The glyph for an icon id; reward icons share a plain diamond. */
export function iconGlyph(id: string): string {
  return GLYPHS[id] ?? (id.startsWith('reward-') ? '◆' : '•');
}
