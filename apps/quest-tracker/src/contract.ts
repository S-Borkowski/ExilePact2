// Reads the fixed name lists out of the brief's `## Export contract` line
// by line (`Frame ids: a, b, ...`), so the tests hold the UI to the brief
// rather than to a second copy of the lists. Test-only.

export function contractList(brief: string, label: string): string[] {
  const line = brief.split(/\r?\n/).find((l) => l.startsWith(`${label}: `));
  if (!line) throw new Error(`the brief has no "${label}:" line`);
  return line
    .slice(label.length + 2)
    .split(',')
    .map((s) => s.trim())
    .filter(Boolean);
}

/** The custom properties a CSS text declares in its `:root` blocks. */
export function rootProperties(css: string): Set<string> {
  const out = new Set<string>();
  for (const block of css.matchAll(/:root\s*\{([^}]*)\}/g)) {
    for (const decl of block[1].matchAll(/(--[a-z0-9-]+)\s*:/g)) out.add(decl[1]);
  }
  return out;
}

/** A token key's custom property: the dot replaced by a dash (`color.panel-bg` -> `--color-panel-bg`). */
export const cssName = (key: string): string => `--${key.replace('.', '-')}`;
