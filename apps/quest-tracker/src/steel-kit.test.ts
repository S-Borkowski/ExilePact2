// The Exile theme against the finished steel/masonry kit
// (`docs/quest-tracker/design/steel-masonry-kit/HANDOFF.md`): the six status
// diamonds drawn from the kit's own geometry, the two kit rasters adopted
// byte for byte, the frame's nine-slice values, and a palette allowlist that
// keeps the rejected warm prototype's colours out of every Exile style file.
// Claude's presentation is pinned by the other suites and is not touched here.
import { afterEach, describe, expect, it } from 'vitest';
import { QUEST_STATUSES, type QuestStatus } from './ipc';
import { STATUS_LABEL } from './labels';
import EXILE from '../theme/exile.json';
import PALETTE from '../../../docs/quest-tracker/design/steel-masonry-kit/palette.json';
import { fakeBridge, fixtureView, readText, renderApp, type Rendered } from './test-support';

const STATUS_SVGS = import.meta.glob('../../../docs/quest-tracker/design/steel-masonry-kit/components/status-*.svg', {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>;
const COMPONENT_SVGS = import.meta.glob('../../../docs/quest-tracker/design/steel-masonry-kit/components/*.svg', {
  query: '?raw',
  import: 'default',
  eager: true,
}) as Record<string, string>;
const SVELTE = import.meta.glob('./**/*.svelte', { query: '?raw', import: 'default', eager: true }) as Record<string, string>;
const TS = import.meta.glob(['./**/*.ts', '!./**/*.test.ts', '!./test-support.ts'], { query: '?raw', import: 'default', eager: true }) as Record<
  string,
  string
>;

const KIT_ASSETS = '../../../docs/quest-tracker/design/steel-masonry-kit/assets/';
const RASTERS = '../theme/materials/raster/';
const RETIRED = ['tracker-steel-frame.png', 'charcoal-stone-well.png', 'journal-frame.png', 'lantern-astrolabe-crest.png'];
/** The paint attributes a drawing element carries, its own or from the group around it. */
const PAINT = ['fill', 'stroke', 'stroke-width', 'stroke-dasharray', 'stroke-linecap', 'opacity'] as const;

let rendered: Rendered | null = null;
afterEach(() => {
  rendered?.destroy();
  rendered = null;
});

/** `status-in-progress.svg` -> `in_progress`. */
function statusOf(path: string): QuestStatus {
  return path.replace(/^.*\/status-([a-z-]+)\.svg$/, '$1').replace('-', '_') as QuestStatus;
}

function kitSvg(status: QuestStatus): Element {
  const entry = Object.entries(STATUS_SVGS).find(([path]) => statusOf(path) === status);
  expect(entry, `kit status-${status}.svg`).toBeDefined();
  const doc = new DOMParser().parseFromString(entry![1], 'image/svg+xml');
  expect(doc.querySelector('parsererror'), status).toBeNull();
  return doc.documentElement;
}

/**
 * Every drawn path of an svg, outside its clip paths, with its geometry, its
 * effective paint and the geometry of the clip that applies to it.
 */
function drawing(svg: Element): Array<Record<string, string | null>> {
  const up = (from: Element, attr: string): Element | null => {
    for (let e: Element | null = from; e; e = e === svg ? null : e.parentElement) if (e.hasAttribute(attr)) return e;
    return null;
  };
  return [...svg.querySelectorAll('path')]
    .filter((path) => !path.closest('clipPath'))
    .map((path) => {
      const out: Record<string, string | null> = { d: path.getAttribute('d') };
      for (const attr of PAINT) out[attr] = up(path, attr)?.getAttribute(attr) ?? null;
      const ref = up(path, 'clip-path')?.getAttribute('clip-path') ?? null;
      const id = ref?.match(/^url\(#([^)]+)\)$/)?.[1];
      const clip = id ? [...svg.querySelectorAll('clipPath')].find((c) => c.getAttribute('id') === id) : undefined;
      out.clip = ref === null ? null : clip ? [...clip.querySelectorAll('path')].map((p) => p.getAttribute('d')).join('|') : `missing ${ref}`;
      return out;
    });
}

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

async function bytes(path: string): Promise<Uint8Array> {
  const fs = (await import(/* @vite-ignore */ 'node:fs' as string)) as { readFileSync(path: URL): Uint8Array };
  return fs.readFileSync(new URL(path, import.meta.url));
}

async function sha256(data: Uint8Array): Promise<string> {
  const crypto = (await import(/* @vite-ignore */ 'node:crypto' as string)) as {
    createHash(name: string): { update(data: Uint8Array): { digest(encoding: string): string } };
  };
  return crypto.createHash('sha256').update(data).digest('hex');
}

describe('the steel/masonry kit in the Exile theme', () => {
  it('reads all six kit status diamonds', () => {
    expect(Object.keys(STATUS_SVGS).map(statusOf).sort()).toEqual([...QUEST_STATUSES].sort());
    for (const status of QUEST_STATUSES) {
      const svg = kitSvg(status);
      expect(svg.getAttribute('viewBox'), status).toBe('0 0 24 24');
      expect(svg.getAttribute('aria-label'), status).toBe(STATUS_LABEL[status]);
      // The kit's only wrapper transform is the identity, so the drawn geometry is the paths' own.
      for (const g of svg.querySelectorAll('[transform]')) expect(g.getAttribute('transform'), status).toMatch(/^translate\(0 0\) scale\(1(?:\.0)?\)$/);
      expect(drawing(svg).length, status).toBeGreaterThan(0);
    }
  });

  it('draws each Exile status mark from the kit geometry, in rows and in detail', async () => {
    const view = fixtureView('panel-next-up', 'play');
    view.settings.theme = 'exile';
    const base = view.next_up[0];
    view.next_up = QUEST_STATUSES.map((status, i) => ({ ...base, id: `kit-${i}`, status }));
    const bridge = fakeBridge(view);
    rendered = await renderApp(bridge);
    function check(mark: Element, status: QuestStatus): void {
      expect(mark.getAttribute('role')).toBe('img');
      expect(mark.getAttribute('aria-label')).toBe(STATUS_LABEL[status]);
      expect(mark.getAttribute('title')).toBe(STATUS_LABEL[status]);
      const svgs = mark.querySelectorAll('svg');
      expect(svgs, status).toHaveLength(1);
      const svg = svgs[0];
      expect(svg.getAttribute('viewBox')).toBe('0 0 24 24');
      expect(svg.getAttribute('aria-hidden')).toBe('true');
      expect(svg.getAttribute('focusable')).toBe('false');
      expect(svg.querySelector('style, title, [role], [aria-label], [transform]')).toBeNull();
      expect(mark.querySelector('.icon, .glyph')).toBeNull();
      expect(drawing(svg), status).toEqual(drawing(kitSvg(status)));
    }
    for (const row of view.next_up) check(rendered.root.querySelector(`[data-quest="${row.id}"] > .status-mark`)!, row.status);
    const detail = fixtureView('quest-detail', 'play');
    detail.settings.theme = 'exile';
    for (const status of QUEST_STATUSES) {
      detail.detail!.row.status = status;
      bridge.pushView(detail);
      check(rendered.root.querySelector('.detail .head > .status-mark')!, status);
    }
    expect(bridge.calls).toEqual([]);
  });

  it('gives every skipped hatch its own clip', async () => {
    const view = fixtureView('panel-next-up', 'play');
    view.settings.theme = 'exile';
    const base = view.next_up[0];
    view.next_up = [0, 1].map((i) => ({ ...base, id: `skip-${i}`, status: 'skipped' as const }));
    rendered = await renderApp(fakeBridge(view));
    const marks = [...rendered.root.querySelectorAll('[data-quest] > .status-mark')];
    expect(marks).toHaveLength(2);
    const ids = marks.map((mark) => {
      const clips = mark.querySelectorAll('clipPath');
      expect(clips).toHaveLength(1);
      const id = clips[0].getAttribute('id')!;
      expect(id).toBeTruthy();
      expect(mark.querySelector('[clip-path]')?.getAttribute('clip-path')).toBe(`url(#${id})`);
      return id;
    });
    expect(new Set(ids).size).toBe(2);
    expect(rendered.root.querySelectorAll(`[id="${ids[0]}"]`)).toHaveLength(1);
  });

  it('adopts the two kit rasters byte for byte and retires the warm runtime art', async () => {
    for (const name of ['frame-neutral-steel.png', 'masonry-charcoal.png']) {
      expect(await sha256(await bytes(`${RASTERS}${name}`)), name).toBe(await sha256(await bytes(`${KIT_ASSETS}${name}`)));
    }
    const frame = await readText('./components/MaterialFrame.svelte');
    expect(frame).toContain('theme/materials/raster/frame-neutral-steel.png');
    for (const path of ['./components/Panel.svelte', './settings/SettingsWindow.svelte']) {
      expect(await readText(path), path).toContain('theme/materials/raster/masonry-charcoal.png');
    }
    const runtime = { ...SVELTE, ...TS, './approved.css': await readText('./approved.css'), '../theme/exile.css': await readText('../theme/exile.css') };
    expect(Object.keys(runtime).length).toBeGreaterThan(20);
    for (const [path, text] of Object.entries(runtime)) {
      for (const name of RETIRED) expect(text.includes(name), `${path} uses ${name}`).toBe(false);
    }
  });

  it('draws the neutral frame as the kit nine-slice: 224 per side, 48 px wide, 8 px out, no centre', async () => {
    const frame = await readText('./components/MaterialFrame.svelte');
    expect(frame).toMatch(/border-image-slice:\s*224;/);
    expect(frame).not.toMatch(/border-image-slice:[^;]*fill/);
    expect(frame).toMatch(/border-image-repeat:\s*stretch;/);
    // A constant size on screen: divided by the zoom the scale setting applies, as the 12 px margin is.
    expect(frame).toMatch(/--frame-zoom:\s*var\(--page-zoom, var\(--preview-scale, 1\)\);/);
    expect(frame).toMatch(/border-image-width:\s*calc\(48px \/ var\(--frame-zoom\)\);/);
    expect(frame).toMatch(/border-image-outset:\s*calc\(8px \/ var\(--frame-zoom\)\);/);
    // The ordinary Settings window has no transparent margin: the plaque sits inside it.
    expect(frame).toMatch(/\.plaque\s*\{[^}]*border-image-outset:\s*0;/);
    expect(frame).toMatch(/pointer-events:\s*none;/);

    rendered = await renderApp(fakeBridge(fixtureView('panel-campaign', 'edit')));
    const art = rendered.root.querySelectorAll('.material-art');
    expect(art).toHaveLength(1);
    expect(art[0].getAttribute('aria-hidden')).toBe('true');
    expect(art[0].hasAttribute('tabindex')).toBe(false);
    expect(art[0].children).toHaveLength(0);
  });

  it('lays the masonry once, as a cover crop over the well, with no warm filter', async () => {
    const css = await readText('./approved.css');
    const well = css.match(/::before\s*\{[^}]*var\(--well-image\)[^}]*\}/)?.[0] ?? '';
    expect(well).toMatch(/var\(--well-image\) center \/ cover no-repeat/);
    expect(well).toMatch(/opacity:\s*calc\(0\.38 \* var\(--panel-opacity, var\(--opacity-default\)\)\)/);
    expect(css).not.toMatch(/sepia|hue-rotate|saturate\(/);
    expect(css).not.toMatch(/background-size:[^;]*\d+px \d+px/);
  });

  it('maps the Exile tokens to the kit palette', () => {
    const exile = EXILE as Record<string, string | number>;
    const expected: Record<string, string> = {
      'color.panel-bg': PALETTE.well,
      'color.panel-sunken': PALETTE.well,
      'color.panel-border': PALETTE.steel,
      'color.hover-outline': PALETTE.edge,
      'color.panel-divider': PALETTE.line,
      'color.text-primary': PALETTE.text,
      'color.text-secondary': PALETTE.muted,
      'color.text-muted': PALETTE.muted,
      'color.worth-none': PALETTE.muted,
      'color.worth-gold': PALETTE.gold,
      'color.edit-accent': PALETTE.ivory,
      'color.action-primary-bg': PALETTE.control,
      'color.row-active-bg': PALETTE.control,
      'color.select-active-bg': PALETTE.control,
      'color.action-primary-hover': PALETTE.hover,
      'color.action-disabled-fg': PALETTE.disabled,
      'color.warning': PALETTE.warn,
      'color.action-danger': PALETTE.error,
      'color.res-fire': '#E99568',
      'color.res-cold': '#91C9DE',
      'color.res-lightning': '#E3C76F',
      'color.res-chaos': '#BA9ACD',
    };
    for (const [key, value] of Object.entries(expected)) expect(String(exile[key]).toUpperCase(), key).toBe(value.toUpperCase());
    // Each status token is a colour of that status's own kit diamond.
    for (const status of QUEST_STATUSES) {
      const kit = new Set(colours(Object.entries(STATUS_SVGS).find(([p]) => statusOf(p) === status)![1].replace(/<style>[\s\S]*?<\/style>/, '')).map(normal));
      const key = `color.status-${status.replace('_', '-')}`;
      expect(kit.has(normal(String(exile[key]))), `${key} ${exile[key]}`).toBe(true);
    }
  });

  it('allows only kit, resistance, black and white colours in the Exile style files', async () => {
    const allowed = new Set<string>(['#000000', '#FFFFFF', '#E99568', '#91C9DE', '#E3C76F', '#BA9ACD']);
    for (const value of Object.values(PALETTE)) allowed.add(normal(value));
    for (const svg of Object.values(COMPONENT_SVGS)) for (const c of colours(svg)) allowed.add(normal(c));
    expect(allowed.size).toBeGreaterThan(30);
    const files = {
      './approved.css': await readText('./approved.css'),
      '../theme/exile.json': await readText('../theme/exile.json'),
      '../theme/exile.css': await readText('../theme/exile.css'),
      './components/MaterialFrame.svelte': await readText('./components/MaterialFrame.svelte'),
      './components/StatusMark.svelte': await readText('./components/StatusMark.svelte'),
    };
    for (const [path, text] of Object.entries(files)) {
      const outside = colours(text).filter((c) => !allowed.has(normal(c)));
      expect(outside, path).toEqual([]);
    }
    // The negative control: the rejected warm gold is caught.
    expect(allowed.has(normal('#EDCF86'))).toBe(false);
  });
});
