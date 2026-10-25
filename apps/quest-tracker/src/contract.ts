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
