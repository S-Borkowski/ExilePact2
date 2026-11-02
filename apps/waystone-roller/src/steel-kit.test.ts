// The Exile theme against the Quest Tracker's finished steel/masonry kit
// (`docs/quest-tracker/design/steel-masonry-kit/HANDOFF.md`): the kit's two
// rasters adopted byte for byte, `theme/exile.css` a byte copy of the Quest
// Tracker's, the frame's nine-slice values, the masonry laid once as a
// cover crop, a palette allowlist over every Exile style file, the theme
// applied to the document from the view and the preview's `theme`
// parameter, and the requested art: a marked placeholder in the DOM for each
// manifest entry, and a loader that draws a delivered file in its place.
// Claude's look is pinned by the other suites and by the runner's legacy
// comparison, not here.
import { afterEach, describe, expect, it } from 'vitest';
import PALETTE from '../../../docs/quest-tracker/design/steel-masonry-kit/palette.json';
import MANIFEST from '../theme/materials/requested/manifest.json';
import { previewBridge } from './bridge';
import { applyRequestedArt, requestedFiles, type ArtRequest } from './requested-art';
import { applyTheme, previewTheme, themeOf } from './theme';
import { fixtureView, readBytes, readText, renderView, type Rendered } from './test-support';

const COMPONENT_SVGS = import.meta.glob('../../../docs/quest-tracker/design/steel-masonry-kit/components/*.svg', {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>;

const KIT_ASSETS = '../../../docs/quest-tracker/design/steel-masonry-kit/assets/';
const RASTERS = '../theme/materials/raster/';
/** The resistance colours the Quest Tracker's `exile.css` carries (byte-copied here). */
const RESISTANCE = ['#E99568', '#91C9DE', '#E3C76F', '#BA9ACD'];

let rendered: Rendered | null = null;
afterEach(() => {
  rendered?.destroy();
  rendered = null;
  delete document.documentElement.dataset.theme;
  for (const attr of [...document.documentElement.attributes]) {
    if (attr.name.startsWith('data-art-')) document.documentElement.removeAttribute(attr.name);
  }
  document.documentElement.removeAttribute('style');
});

/** A colour literal as `#RRGGBB`, alpha dropped. */
function normal(literal: string): string {
  const hex = literal.match(/^#([0-9a-f]{3,8})$/i)?.[1];
  if (hex) {
    const full = hex.length <= 4 ? [...hex.slice(0, 3)].map((c) => c + c).join('') : hex.slice(0, 6);
    return `#${full.toUpperCase()}`;
  }
  const parts = literal.replace(/^rgba?\(|\)$/g, '').split(/[\s,/]+/).filter(Boolean).slice(0, 3);
  return `#${parts.map((p) => Math.round(p.endsWith('%') ? (parseFloat(p) * 255) / 100 : parseFloat(p)).toString(16).padStart(2, '0')).join('').toUpperCase()}`;
}

function colours(text: string): string[] {
  return [...text.matchAll(/#[0-9a-f]{3,8}\b|rgba?\([^)]*\)/gi)].map((m) => m[0]);
}

const same = (a: Uint8Array, b: Uint8Array) => a.length === b.length && a.every((byte, i) => byte === b[i]);

/** A selector list split at its top-level commas (those inside `:is(...)` stay). */
function splitList(list: string): string[] {
  const parts: string[] = [];
  let depth = 0;
  let start = 0;
  for (let i = 0; i < list.length; i++) {
    if (list[i] === '(') depth++;
    else if (list[i] === ')') depth--;
    else if (list[i] === ',' && depth === 0) {
      parts.push(list.slice(start, i));
      start = i + 1;
    }
  }
  parts.push(list.slice(start));
  return parts.map((s) => s.trim()).filter(Boolean);
}

/** Every selector of a stylesheet's rules (comments and at-rule preludes dropped), one per list entry. */
function selectors(css: string): string[] {
  const text = css.replace(/\/\*[\s\S]*?\*\//g, '');
  const out: string[] = [];
  const re = /([^{}]+)\{/g;
  for (let m = re.exec(text); m; m = re.exec(text)) {
    const prelude = m[1].trim();
    if (!prelude || prelude.startsWith('@') || /^(from|to|\d+%)$/.test(prelude)) continue;
    out.push(...splitList(prelude));
  }
  return out;
}

describe('the steel/masonry kit in the Exile theme', () => {
  it('adopts the two kit rasters byte for byte', async () => {
    for (const name of ['frame-neutral-steel.png', 'masonry-charcoal.png']) {
      const ours = await readBytes(`${RASTERS}${name}`);
      expect(ours.length, name).toBeGreaterThan(10_000);
      expect(same(ours, await readBytes(`${KIT_ASSETS}${name}`)), name).toBe(true);
    }
    expect(await readText('./components/MaterialFrame.svelte')).toContain('theme/materials/raster/frame-neutral-steel.png');
    for (const path of ['./App.svelte', './settings/SettingsWindow.svelte']) {
      expect(await readText(path), path).toContain('theme/materials/raster/masonry-charcoal.png');
    }
  });

  it('copies exile.css from the quest tracker byte for byte', async () => {
    const ours = await readBytes('../theme/exile.css');
    const theirs = await readBytes('../../quest-tracker/theme/exile.css');
    expect(ours.length).toBeGreaterThan(1000);
    expect(same(ours, theirs), "theme/exile.css differs from the quest tracker's").toBe(true);
    // Negative control: a one-byte change is seen.
    const changed = ours.slice();
    changed[changed.length - 3] ^= 1;
    expect(same(changed, theirs)).toBe(false);
    // Both pages load it after the tokens, and steel.css after their own rules.
    const main = await readText('./main.ts');
    expect(main.indexOf('theme/exile.css')).toBeGreaterThan(main.indexOf('theme/tokens.css'));
    expect(main.indexOf('./steel.css')).toBeGreaterThan(main.indexOf('./app.css'));
    const settings = await readText('./settings/main.ts');
    expect(settings.indexOf('theme/exile.css')).toBeGreaterThan(settings.indexOf('theme/tokens.css'));
    expect(settings.indexOf('../steel.css')).toBeGreaterThan(settings.indexOf('./settings.css'));
  });

  it('draws the neutral frame as the kit nine-slice: 224 per side, 48 px wide, flush outsets, no centre', async () => {
    const frame = await readText('./components/MaterialFrame.svelte');
    expect(frame).toMatch(/border-image-slice:\s*224;/);
    expect(frame).not.toMatch(/border-image-slice:[^;]*fill/);
    expect(frame).toMatch(/border-image-repeat:\s*stretch;/);
    expect(frame).toMatch(/border-image-width:\s*48px;/);
    // Top, right, bottom, left: just under the PNG's own padding at 48 px
    // (9.4 / 8.4 / 13.7 / 9 px), so the art meets every window edge.
    expect(frame).toMatch(/border-image-outset:\s*9px 8px 13\.5px 8\.5px;/);
    expect(frame).toMatch(/pointer-events:\s*none;/);
    // Fixed to the window, not to a scroller.
    expect(frame).toMatch(/position:\s*fixed;/);

    // One decorative frame in Exile, hidden from assistive technology and never focused.
    rendered = await renderView({ ...fixtureView('ok-any'), theme: 'exile' });
    const art = rendered.root.querySelectorAll('.material-art');
    expect(art).toHaveLength(1);
    expect(art[0].getAttribute('aria-hidden')).toBe('true');
    expect(art[0].hasAttribute('tabindex')).toBe(false);
    expect(art[0].children).toHaveLength(0);
    rendered.destroy();

    // None in Claude: today's page, as it was.
    rendered = await renderView({ ...fixtureView('ok-any'), theme: 'claude' });
    expect(rendered.root.querySelectorAll('.material-art')).toHaveLength(0);
  });

  it('lays the masonry once, as a cover crop over the well, with no warm filter', async () => {
    const css = await readText('./steel.css');
    const wells = css.match(/::before\s*\{[^}]*var\(--well-image\)[^}]*\}/g) ?? [];
    expect(wells).toHaveLength(1);
    expect(wells[0]).toMatch(/var\(--well-image\) center \/ cover no-repeat/);
    expect(wells[0]).toMatch(/opacity:\s*0\.38;/);
    expect(css).not.toMatch(/sepia|hue-rotate|saturate\(/);
    expect(css).not.toMatch(/background-repeat:\s*repeat/);
    expect(css).not.toMatch(/background-size:[^;]*\d+px \d+px/);
  });

  it('scopes every steel rule to the Exile theme', async () => {
    const all = selectors(await readText('./steel.css'));
    expect(all.length).toBeGreaterThan(40);
    const loose = all.filter((s) => !s.startsWith(":root[data-theme='exile']"));
    expect(loose).toEqual([]);
    // Negative controls: an unscoped rule is caught, and so is the unscoped
    // half of a list, but not a comma inside `:is()`.
    const unscoped = (css: string) => selectors(css).filter((s) => !s.startsWith(":root[data-theme='exile']"));
    expect(unscoped('.btn { color: red; }')).toEqual(['.btn']);
    expect(unscoped(":root[data-theme='exile'] .a, .b { color: red; }")).toEqual(['.b']);
    expect(unscoped(":root[data-theme='exile'] :is(.a, .b) { color: red; }")).toEqual([]);
  });

  it('allows only kit, resistance, black and white colours in the Exile style files', async () => {
    const allowed = new Set<string>(['#000000', '#FFFFFF', ...RESISTANCE]);
    for (const value of Object.values(PALETTE)) allowed.add(normal(value));
    for (const svg of Object.values(COMPONENT_SVGS)) for (const c of colours(svg)) allowed.add(normal(c));
    expect(allowed.size).toBeGreaterThan(30);
    const files: Record<string, string> = {
      '../theme/exile.css': await readText('../theme/exile.css'),
      './steel.css': await readText('./steel.css'),
      './components/MaterialFrame.svelte': await readText('./components/MaterialFrame.svelte'),
      './settings/AppearanceSection.svelte': await readText('./settings/AppearanceSection.svelte'),
    };
    for (const [path, text] of Object.entries(files)) {
      expect(text.length, path).toBeGreaterThan(100);
      const outside = colours(text).filter((c) => !allowed.has(normal(c)));
      expect(outside, path).toEqual([]);
    }
    // Controls are steel grey, never cyan or gold fills; focus is the ivory ring.
    const steel = await readText('./steel.css');
    expect(steel).toMatch(/#424D54/i);
    expect(steel).toMatch(/#536169/i);
    expect(steel).toMatch(/outline:\s*2px solid var\(--color-edit-accent\)/);
    // Negative controls: the rejected warm gold and the Claude cyan accent are caught.
    expect(allowed.has(normal('#EDCF86'))).toBe(false);
    expect(allowed.has(normal('#4FC3D9'))).toBe(false);
  });

  it('applies the theme from the view, exile until a view says claude', async () => {
    // The pure rule: only exactly "claude" is Claude.
    expect(themeOf('claude')).toBe('claude');
    for (const v of ['exile', undefined, null, 'Claude', 'dark', 3]) expect(themeOf(v), String(v)).toBe('exile');
    const root = document.createElement('div');
    applyTheme(root, undefined);
    expect(root.dataset.theme).toBe('exile');
    applyTheme(root, 'claude');
    expect(root.dataset.theme).toBe('claude');

    // The page sets the document's theme from the view it receives.
    rendered = await renderView({ ...fixtureView('ok-any'), theme: 'claude' });
    expect(document.documentElement.dataset.theme).toBe('claude');
    rendered.bridge.pushView({ ...fixtureView('ok-any'), theme: 'exile' });
    expect(document.documentElement.dataset.theme).toBe('exile');
    rendered.destroy();
    rendered = null;

    // A view without a theme (an older shell) is Exile.
    const old = fixtureView('ok-any') as unknown as Record<string, unknown>;
    delete old.theme;
    rendered = await renderView(old as never);
    expect(document.documentElement.dataset.theme).toBe('exile');
  });

  it('lets the preview choose the theme over the fixture', async () => {
    expect(previewTheme('')).toBeNull();
    expect(previewTheme('?theme=claude')).toBe('claude');
    expect(previewTheme('?view=too-long&theme=exile')).toBe('exile');
    expect(previewTheme('?theme=dark')).toBeNull();

    const views: Array<{ theme?: string }> = [];
    await previewBridge('?view=too-long&theme=claude').start((v) => views.push(v));
    expect(views.map((v) => v.theme)).toEqual(['claude']);
    views.length = 0;
    await previewBridge('?view=too-long&theme=exile').start((v) => views.push(v));
    expect(views.map((v) => v.theme)).toEqual(['exile']);
    // With no parameter the fixture's own theme stands.
    views.length = 0;
    await previewBridge('?view=too-long').start((v) => views.push(v));
    expect(views.map((v) => v.theme)).toEqual([fixtureView('too-long').theme]);
  });
});

describe('the requested art', () => {
  const requests = MANIFEST.requests as ArtRequest[];

  it('lists each request once, delivered, with wr- ids and file names', () => {
    expect(requests.length).toBeGreaterThanOrEqual(3);
    const ids = requests.map((r) => r.id);
    expect(new Set(ids).size).toBe(ids.length);
    for (const r of requests) {
      expect(r.id, r.id).toMatch(/^wr-[a-z0-9-]+$/);
      expect(r.status, r.id).toBe('delivered');
      expect(r.files.length, r.id).toBeGreaterThan(0);
      for (const f of r.files) expect(f, r.id).toMatch(/^[a-z0-9-]+\.(png|svg)$/);
      expect(r.element.length, r.id).toBeGreaterThan(10);
    }
    expect(ids).toEqual(expect.arrayContaining(['wr-mark-want', 'wr-mark-avoid', 'wr-update-dot']));
  });

  it('marks a placeholder in the DOM for every request, on a fixture that shows it', async () => {
    // ok-any wants and avoids mods; an update is known in the second view.
    const view = { ...fixtureView('ok-any'), theme: 'exile' as const, update_available: '1.3.0' };
    rendered = await renderView(view);
    const shown = new Set([...rendered.root.querySelectorAll('[data-art-placeholder]')].map((el) => el.getAttribute('data-art-placeholder')));
    rendered.destroy();
    // too-long shows the parts list, with its want and avoid parts.
    rendered = await renderView({ ...fixtureView('too-long'), theme: 'exile' });
    for (const el of rendered.root.querySelectorAll('[data-art-placeholder]')) shown.add(el.getAttribute('data-art-placeholder'));
    for (const r of requests) expect(shown.has(r.id), `${r.id} has no placeholder in the DOM`).toBe(true);
    // And nothing marked that the manifest does not list.
    for (const id of shown) expect(requests.some((r) => r.id === id), `${id} is not in the manifest`).toBe(true);

    // The marks keep their words and glyphs: a placeholder changes no text.
    const row = rendered.root.querySelector('.mod[data-choice="want"] .choice-mark');
    expect(row?.getAttribute('data-art-placeholder')).toBe('wr-mark-want');
    expect(row?.textContent).toBe('+');
  });

  it('draws a delivered file in place of its placeholder, and nothing while it is absent', () => {
    const root = document.createElement('div');
    // No file: no request is wired, the placeholder stays.
    expect(applyRequestedArt(root, {}, requests)).toEqual([]);
    expect([...root.attributes].map((a) => a.name)).toEqual([]);

    // A file present under its listed name: its request is marked and its URL set.
    const files = { '../theme/materials/requested/update-dot.svg': '/assets/update-dot-abc.svg' };
    expect(applyRequestedArt(root, files, requests)).toEqual(['wr-update-dot']);
    expect(root.hasAttribute('data-art-wr-update-dot')).toBe(true);
    expect(root.style.getPropertyValue('--art-update-dot')).toBe('url("/assets/update-dot-abc.svg")');
    // The others are untouched.
    expect(root.hasAttribute('data-art-wr-mark-want')).toBe(false);

    // A request with several files is wired only when every one is there.
    const pair: ArtRequest[] = [{ id: 'wr-pair', files: ['a.svg', 'b.svg'], element: 'test pair element', status: 'placeholder' }];
    const half = document.createElement('div');
    expect(applyRequestedArt(half, { '../theme/materials/requested/a.svg': '/a.svg' }, pair)).toEqual([]);
    expect(half.hasAttribute('data-art-wr-pair')).toBe(false);
    expect(applyRequestedArt(half, { '../theme/materials/requested/a.svg': '/a.svg', '../theme/materials/requested/b.svg': '/b.svg' }, pair)).toEqual(['wr-pair']);
  });

  it('finds exactly the delivered files, and wires every request from them', () => {
    // The shipped folder holds the owner's three files and the manifest.
    const names = Object.keys(requestedFiles()).map((p) => p.replace(/^.*\//, '')).sort();
    expect(names).toEqual(requests.flatMap((r) => r.files).sort());
    expect(names).toEqual(['mark-avoid.svg', 'mark-want.svg', 'update-dot.svg']);
    // With the real files, every request is wired and each file's URL is set.
    const root = document.createElement('div');
    expect(applyRequestedArt(root)).toEqual(requests.map((r) => r.id));
    for (const r of requests) {
      expect(root.hasAttribute(`data-art-${r.id}`), r.id).toBe(true);
      for (const f of r.files) expect(root.style.getPropertyValue(`--art-${f.replace(/\.(png|svg)$/, '')}`), f).toMatch(/^url\(".+"\)$/);
    }
  });

  it('draws the placeholder only in Exile, and the art over it once delivered', async () => {
    const css = await readText('./steel.css');
    for (const r of requests) {
      expect(css, r.id).toContain(`[data-art-placeholder='${r.id}']`);
      expect(css, r.id).toContain(`[data-art-${r.id}]`);
      for (const f of r.files) expect(css, `${r.id} ${f}`).toContain(`var(--art-${f.replace(/\.(png|svg)$/, '')})`);
    }
  });
});
