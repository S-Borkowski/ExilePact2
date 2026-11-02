// The Exile theme against the quest tracker's finished steel/masonry kit
// (`docs/quest-tracker/design/steel-masonry-kit/HANDOFF.md`): the two kit
// rasters and the Exile token file adopted byte for byte, the frame's
// nine-slice values, the masonry laid once as a cover crop, a palette
// allowlist over every Exile style file, the theme taken from the view (and
// from a preview's `theme` parameter) on all three pages, and the
// requested-art placeholders with the loader that swaps in a delivered file.
// Claude's presentation is pinned by the other suites and is not touched here.
import { afterEach, describe, expect, it } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import PALETTE from '../../../docs/quest-tracker/design/steel-masonry-kit/palette.json';
import MANIFEST from '../theme/materials/requested/manifest.json';
import Overlay from './overlay/Overlay.svelte';
import SettingsWindow from './settings/SettingsWindow.svelte';
import { previewBridge } from './bridge';
import type { View } from './ipc';
import { applyRequestedArt, requestedFiles, type RequestedManifest } from './requested-art';
import { applyTheme, previewTheme } from './theme';
import { fakeBridge, fakeSettingsBridge, fixtureView, readBytes, renderComponent, renderView, settingsFixture, type FakeBridge, type Rendered } from './test-support';

const COMPONENT_SVGS = import.meta.glob('../../../docs/quest-tracker/design/steel-masonry-kit/components/*.svg', {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>;
const SVELTE = import.meta.glob('./**/*.svelte', { query: '?raw', import: 'default', eager: true }) as Record<string, string>;

const KIT = '../../../docs/quest-tracker/design/steel-masonry-kit/';
const RASTERS = '../theme/materials/raster/';

const decoder = new TextDecoder();
const readText = async (path: string): Promise<string> => decoder.decode(await readBytes(path));
const same = (a: Uint8Array, b: Uint8Array) => a.length === b.length && a.every((byte, i) => byte === b[i]);

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
  return [...text.replace(/\/\*[\s\S]*?\*\//g, '').matchAll(/#[0-9a-f]{3,8}\b|rgba?\([^)]*\)/gi)].map((m) => m[0]);
}

/** Every rule's selector list in a stylesheet, comments and at-rule wrappers dropped. */
function selectors(css: string): string[] {
  const plain = css.replace(/\/\*[\s\S]*?\*\//g, '');
  const out: string[] = [];
  for (const m of plain.matchAll(/([^{}]+)\{/g)) {
    const head = m[1].trim();
    if (!head || head.startsWith('@')) continue;
    // Split on the commas between selectors, not those inside `:is(...)`.
    let depth = 0;
    let from = 0;
    for (let i = 0; i <= head.length; i++) {
      const c = head[i];
      if (c === '(') depth++;
      else if (c === ')') depth--;
      else if ((c === ',' && depth === 0) || i === head.length) {
        const one = head.slice(from, i).trim();
        if (one) out.push(one);
        from = i + 1;
      }
    }
  }
  return out;
}

const EXILE = ":root[data-theme='exile']";

/** The declarations of every top-level steel.css rule naming `EXILE target` (or `EXILE` alone), the last one winning. */
function declared(css: string, target: string): Record<string, string> {
  const want = target ? `${EXILE} ${target}` : EXILE;
  const out: Record<string, string> = {};
  for (const m of css.replace(/\/\*[\s\S]*?\*\//g, '').matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
    if (!selectors(`${m[1]}{`).includes(want)) continue;
    for (const d of m[2].split(';')) {
      const i = d.indexOf(':');
      if (i > 0) out[d.slice(0, i).trim()] = d.slice(i + 1).trim();
    }
  }
  return out;
}

/** The alpha of a translucent black `rgba(0, 0, 0, a)`, or null for anything else. */
function blackAlpha(value: string | undefined): number | null {
  const a = value?.match(/^rgba\(\s*0\s*,\s*0\s*,\s*0\s*,\s*([0-9.]+)\s*\)$/)?.[1];
  return a === undefined ? null : Number(a);
}

let rendered:(Rendered & { bridge: FakeBridge }) | null = null;
let others: Array<() => void> = [];

afterEach(() => {
  rendered?.destroy();
  rendered = null;
  for (const d of others) d();
  others = [];
  delete document.documentElement.dataset.theme;
  for (const name of [...document.documentElement.attributes].map((a) => a.name)) {
    if (name.startsWith('data-art-')) document.documentElement.removeAttribute(name);
  }
  document.documentElement.removeAttribute('style');
});

describe('the steel/masonry kit in the Exile theme', () => {
  it('adopts the two kit rasters and the Exile token file byte for byte', async () => {
    for (const name of ['frame-neutral-steel.png', 'masonry-charcoal.png']) {
      const ours = await readBytes(`${RASTERS}${name}`);
      expect(ours.length, name).toBeGreaterThan(1000);
      expect(same(ours, await readBytes(`${KIT}assets/${name}`)), name).toBe(true);
    }
    const exile = await readBytes('../theme/exile.css');
    const theirs = await readBytes('../../quest-tracker/theme/exile.css');
    expect(exile.length).toBeGreaterThan(1000);
    expect(same(exile, theirs), "theme/exile.css differs from the quest tracker's").toBe(true);
    // Negative control: the comparison sees a one-byte change.
    const changed = exile.slice();
    changed[changed.length - 3] ^= 1;
    expect(same(changed, theirs)).toBe(false);
    // Each page imports the tokens, then exile.css, then its own sheet, then steel.css.
    for (const [path, own] of [
      ['./main.ts', './app.css'],
      ['./settings/main.ts', './settings.css'],
      ['./overlay/main.ts', './overlay.css'],
    ] as const) {
      const text = await readText(path);
      // The import lines alone, not the comments above them.
      const imports = text.split(/\r?\n/).filter((line) => line.startsWith('import ')).join('\n');
      const at = (needle: string) => imports.indexOf(needle);
      expect(at('theme/tokens.css'), path).toBeGreaterThan(-1);
      expect(at('theme/exile.css'), path).toBeGreaterThan(at('theme/tokens.css'));
      expect(at(own), path).toBeGreaterThan(at('theme/exile.css'));
      expect(at('steel.css'), path).toBeGreaterThan(at(own));
    }
  });

  it('draws the neutral frame as the kit nine-slice: 224 per side, 48 px wide, flush outsets, no centre', async () => {
    const frame = await readText('./components/MaterialFrame.svelte');
    expect(frame).toContain('theme/materials/raster/frame-neutral-steel.png');
    expect(frame).toMatch(/border-image-slice:\s*224;/);
    expect(frame).not.toMatch(/border-image-slice:[^;]*fill/);
    expect(frame).toMatch(/border-image-repeat:\s*stretch;/);
    expect(frame).toMatch(/border-image-width:\s*48px;/);
    // Top, right, bottom, left: just under the PNG's own padding at 48 px.
    expect(frame).toMatch(/border-image-outset:\s*9px 8px 13\.5px 8\.5px;/);
    expect(frame).toMatch(/pointer-events:\s*none;/);
    expect(frame).toMatch(/position:\s*fixed;/);

    // One frame on the main window and the Settings window, none in Claude.
    rendered = await renderView(fixtureView('craft-planning'));
    let art = rendered.root.querySelectorAll('.material-art');
    expect(art).toHaveLength(1);
    expect(art[0].getAttribute('aria-hidden')).toBe('true');
    expect(art[0].hasAttribute('tabindex')).toBe(false);
    expect(art[0].children).toHaveLength(0);
    rendered.bridge.pushView({ ...fixtureView('craft-planning'), theme: 'claude' });
    expect(rendered.root.querySelectorAll('.material-art')).toHaveLength(0);

    const settings = await renderComponent(SettingsWindow, { bridge: { ...fakeSettingsBridge(settingsFixture('idle')), catalogue: async () => [] } }, (root) =>
      root.querySelector('.window[data-state]:not([data-state="none"])') !== null,
    );
    others.push(settings.destroy);
    art = settings.root.querySelectorAll('.material-art');
    expect(art).toHaveLength(1);
    expect(art[0].getAttribute('aria-hidden')).toBe('true');
  });

  it('lays the masonry once per well, as a cover crop with no repeat and no warm filter', async () => {
    const css = await readText('./steel.css');
    const wells = [...css.matchAll(/\{[^}]*var\(--well-image\)[^}]*\}/g)].map((m) => m[0]);
    expect(wells.length).toBeGreaterThan(0);
    for (const well of wells) {
      expect(well).toMatch(/var\(--well-image\) center \/ cover no-repeat/);
      expect(well).toMatch(/opacity:\s*0\.38/);
    }
    expect(css).not.toMatch(/sepia|hue-rotate|saturate\(/);
    expect(css).not.toMatch(/background-repeat:\s*repeat/);
    // Each page hands its well the kit masonry, in Exile only.
    for (const path of ['./App.svelte', './settings/SettingsWindow.svelte', './overlay/Overlay.svelte']) {
      expect(SVELTE[path], path).toContain('theme/materials/raster/masonry-charcoal.png');
    }
  });

  it('scopes every steel.css rule to the Exile theme', async () => {
    const css = await readText('./steel.css');
    const list = selectors(css);
    expect(list.length).toBeGreaterThan(40);
    for (const s of list) expect(s, 'a steel.css selector').toMatch(/^:root\[data-theme='exile'\]/);
  });

  it('allows only kit, resistance, black and white colours in the Exile style files', async () => {
    // The resistance colours are the quest tracker's reward tokens, carried by the byte copy of exile.css.
    const allowed = new Set<string>(['#000000', '#FFFFFF', '#E99568', '#91C9DE', '#E3C76F', '#BA9ACD']);
    for (const value of Object.values(PALETTE)) allowed.add(normal(value));
    for (const svg of Object.values(COMPONENT_SVGS)) for (const c of colours(svg)) allowed.add(normal(c));
    expect(allowed.size).toBeGreaterThan(30);
    const files: Record<string, string> = {
      '../theme/exile.css': await readText('../theme/exile.css'),
      './steel.css': await readText('./steel.css'),
      './components/MaterialFrame.svelte': SVELTE['./components/MaterialFrame.svelte'],
    };
    for (const [path, text] of Object.entries(files)) {
      expect(text, path).toBeTruthy();
      const outside = colours(text).filter((c) => !allowed.has(normal(c)));
      expect(outside, path).toEqual([]);
    }
    // Negative controls: Claude's cyan accent and a warm gold are caught.
    expect(allowed.has(normal('#EDCF86'))).toBe(false);
    expect(allowed.has(normal('#5BC0EB'))).toBe(false);
    // Controls take steel-grey fills, never gold: no `background` of the gold token.
    const css = files['./steel.css'];
    expect(css).not.toMatch(/background(-color)?:[^;]*(--color-worth-gold|#D0BF94)/i);
  });

  it('lets the masonry show through every section panel, and edges each modifier list in steel', async () => {
    // The owner's Live 2 look check: the grabbed item's sections read as
    // opaque slabs and the modifier checklist had no border.
    const css = await readText('./steel.css');
    const alpha = blackAlpha(declared(css, '')['--section-fill']);
    expect(alpha, '--section-fill is translucent black').not.toBeNull();
    expect(alpha!).toBeGreaterThan(0);
    expect(alpha!).toBeLessThan(1);
    const panels = ['.chosen', '.blocked', '.card', "[data-art-placeholder='ch-item-card']", '.choice-list', '.mod-list', '.picker .list'];
    for (const panel of panels) {
      expect(declared(css, panel).background, panel).toBe('var(--section-fill)');
    }
    // The lists carry the steel edge at every width, not only below 880 px.
    for (const list of ['.choice-list', '.mod-list', '.picker .list']) {
      expect(declared(css, list).border, list).toBe('1px solid var(--color-panel-divider)');
    }
    for (const box of ['.chosen', '.blocked', '.card']) {
      expect(declared(css, box)['border-color'], box).toBe('var(--color-panel-divider)');
    }
    // Negative controls: the opaque slate fill, and a rule for another panel, are not taken.
    expect(blackAlpha('#121B20')).toBeNull();
    expect(declared(`${EXILE} .other { background: var(--section-fill); }`, '.card').background).toBeUndefined();
  });

  it('takes the theme from each view on all three pages, exile before the first', async () => {
    const root = document.documentElement;
    root.dataset.theme = 'claude';
    applyTheme(root, undefined);
    expect(root.dataset.theme).toBe('exile');

    rendered = await renderView(fixtureView('craft-planning'));
    expect(root.dataset.theme).toBe('exile');
    rendered.bridge.pushView({ ...fixtureView('craft-planning'), theme: 'claude' });
    expect(root.dataset.theme).toBe('claude');
    // A view without a theme (an older shell) is Exile.
    const old = fixtureView('craft-planning') as Partial<View>;
    delete old.theme;
    rendered.bridge.pushView(old as View);
    expect(root.dataset.theme).toBe('exile');
    rendered.destroy();
    rendered = null;

    const overlay = fakeBridge({ ...fixtureView('overlay-active'), theme: 'claude' });
    const target = document.createElement('div');
    document.body.append(target);
    const app = mount(Overlay, { target, props: { bridge: overlay } });
    others.push(() => {
      unmount(app);
      target.remove();
    });
    flushSync();
    for (let i = 0; i < 100 && !target.querySelector('.panel'); i++) {
      await new Promise((r) => setTimeout(r, 5));
      flushSync();
    }
    expect(root.dataset.theme).toBe('claude');
    overlay.pushView(fixtureView('overlay-active'));
    expect(root.dataset.theme).toBe('exile');
  });

  it('lets a preview choose the theme with its theme parameter', async () => {
    expect(previewTheme('?theme=claude')).toBe('claude');
    expect(previewTheme('?state=empty&theme=exile')).toBe('exile');
    expect(previewTheme('?theme=sepia')).toBeNull();
    expect(previewTheme('')).toBeNull();
    let shown: View | null = null;
    await previewBridge('?state=craft-planning&theme=claude').start((v) => {
      shown = v;
    });
    expect(shown!.theme).toBe('claude');
    await previewBridge('?state=craft-planning').start((v) => {
      shown = v;
    });
    expect(shown!.theme).toBe('exile');
  });
});

describe('requested art', () => {
  const manifest = MANIFEST as RequestedManifest;

  it('lists each request once, every one delivered', () => {
    const ids = manifest.requests.map((r) => r.id);
    expect(ids).toEqual(['ch-radio', 'ch-overlay-frame', 'ch-item-card', 'ch-step-marker', 'ch-update-dot']);
    expect(new Set(ids).size).toBe(ids.length);
    for (const r of manifest.requests) {
      expect(r.id, r.id).toMatch(/^ch-[a-z0-9-]+$/);
      expect(r.status, r.id).toBe('delivered');
      expect(r.files.length, r.id).toBeGreaterThan(0);
      for (const f of r.files) expect(f, r.id).toMatch(/^[a-z0-9-]+\.(png|svg)$/);
      expect(r.element.length, r.id).toBeGreaterThan(10);
    }
  });

  it('draws a placeholder for every request in a fixture that shows its element', async () => {
    const shows: Record<string, () => Promise<HTMLElement>> = {
      'ch-radio': async () => (rendered = await renderView(fixtureView('trade-picking'))).root,
      'ch-item-card': async () => (rendered = await renderView(fixtureView('craft-planning'))).root,
      'ch-step-marker': async () => (rendered = await renderView(fixtureView('craft-planning'))).root,
      'ch-update-dot': async () => (rendered = await renderView({ ...fixtureView('empty'), update_available: '0.4.0' })).root,
      'ch-overlay-frame': async () => {
        const r = await renderComponent(Overlay, { bridge: fakeBridge(fixtureView('overlay-active')) }, (root) => root.querySelector('.panel') !== null);
        others.push(r.destroy);
        return r.root;
      },
    };
    for (const r of manifest.requests) {
      expect(shows[r.id], r.id).toBeDefined();
      const root = await shows[r.id]();
      expect(root.querySelectorAll(`[data-art-placeholder="${r.id}"]`).length, r.id).toBeGreaterThan(0);
      rendered?.destroy();
      rendered = null;
    }
    // Negative control: a request nobody draws is not found.
    const root = await shows['ch-radio']();
    expect(root.querySelectorAll('[data-art-placeholder="ch-nothing"]')).toHaveLength(0);
  });

  it('swaps a delivered file in by path: absent file, placeholder; present file, art', () => {
    const root = document.createElement('div');
    // Nothing delivered: no request is marked, no art property is set.
    expect(applyRequestedArt(root, {}, manifest)).toEqual([]);
    expect([...root.attributes].map((a) => a.name)).toEqual([]);
    // One of ch-radio's four files: not all of its states, so still the placeholder.
    const one = { '../theme/materials/requested/radio-off.svg': '/assets/radio-off.svg' };
    expect(applyRequestedArt(root, one, manifest)).toEqual([]);
    expect(root.hasAttribute('data-art-ch-radio')).toBe(false);
    expect(root.style.getPropertyValue('--art-radio-off')).toBe('url("/assets/radio-off.svg")');
    // Every file of a request: that request is marked and its files are reachable as properties.
    const all = {
      '../theme/materials/requested/update-dot.svg': '/assets/update-dot.svg',
      '../theme/materials/requested/item-card-frame.png': '/assets/item-card-frame.png',
    };
    expect(applyRequestedArt(root, all, manifest)).toEqual(['ch-item-card', 'ch-update-dot']);
    expect(root.hasAttribute('data-art-ch-update-dot')).toBe(true);
    expect(root.hasAttribute('data-art-ch-item-card')).toBe(true);
    expect(root.style.getPropertyValue('--art-update-dot')).toBe('url("/assets/update-dot.svg")');
  });

  it('finds exactly the delivered files, and marks every request from them', () => {
    // The shipped folder holds the owner's eight files and the manifest.
    const names = Object.keys(requestedFiles()).map((p) => p.replace(/^.*\//, '')).sort();
    expect(names).toEqual(manifest.requests.flatMap((r) => r.files).sort());
    expect(names).toHaveLength(8);
    // With the real files, every request is marked and each file's URL is set.
    const root = document.createElement('div');
    expect(applyRequestedArt(root)).toEqual(manifest.requests.map((r) => r.id));
    for (const r of manifest.requests) {
      expect(root.hasAttribute(`data-art-${r.id}`), r.id).toBe(true);
      for (const f of r.files) expect(root.style.getPropertyValue(`--art-${f.replace(/\.(png|svg)$/, '')}`), f).toMatch(/^url\(".+"\)$/);
    }
  });

  it('layers the focus ring over the radio\'s off or on base, never in place of it', async () => {
    const css = (await readText('./steel.css')).replace(/\s+/g, ' ');
    const at = ":root[data-theme='exile'][data-art-ch-radio] [data-art-placeholder='ch-radio']";
    expect(css).toContain(`${at}:focus-visible { background-image: var(--art-radio-focus), var(--art-radio-off);`);
    expect(css).toContain(`${at}:checked:focus-visible { background-image: var(--art-radio-focus), var(--art-radio-on);`);
    // Negative control: the focus image alone, which hid the selection, is gone.
    expect(css).not.toMatch(/background-image: var\(--art-radio-focus\);/);
  });

  it('has Exile rules that draw each delivered file in place of its placeholder', async () => {
    const css = await readText('./steel.css');
    for (const r of manifest.requests) {
      expect(css, r.id).toContain(`[data-art-placeholder='${r.id}']`);
      expect(css, r.id).toContain(`[data-art-${r.id}]`);
      for (const f of r.files) expect(css, `${r.id} ${f}`).toContain(`var(--art-${f.replace(/\.(png|svg)$/, '')})`);
    }
  });
});
