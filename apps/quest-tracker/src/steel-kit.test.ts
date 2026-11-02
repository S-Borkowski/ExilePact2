// The Exile theme against the finished steel/masonry kit
// (`docs/quest-tracker/design/steel-masonry-kit/HANDOFF.md`): the six status
// diamonds drawn from the kit's own geometry, the two kit rasters adopted
// byte for byte, the frame's nine-slice values, a palette allowlist that
// keeps the rejected warm prototype's colours out of every Exile style file,
// and Next up's worth sections (a labelled gold heading, the kit divider
// between sections, a fine line between the quests of one).
// Claude's presentation is pinned by the other suites and is not touched here.
import { afterEach, describe, expect, it } from 'vitest';
import { QUEST_STATUSES, type QuestStatus, type Worth } from './ipc';
import { STATUS_LABEL, WORTH_ICON, WORTH_LABEL } from './labels';
import { sectionsView } from './preview';
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
/** Every kit diamond's outer ring. */
const RING = 'M12 2 22 12 12 22 2 12Z';
/** The statuses whose kit ring is filled, and that fill: drawn inside the ring instead (owner, Live 2). */
const INSIDE_FILL: Partial<Record<QuestStatus, string>> = { in_progress: '#E6E5DE', done: '#818D95' };
/** How far an absolute polygon path reaches from the diamond's centre, in the diamond's own (L1) measure. */
function radius(d: string): number {
  expect(d, d).toMatch(/^M[\d.\s]+Z$/);
  const n = d.slice(1, -1).trim().split(/\s+/).map(Number);
  let most = 0;
  for (let i = 0; i + 1 < n.length; i += 2) most = Math.max(most, Math.abs(n[i] - 12) + Math.abs(n[i + 1] - 12));
  return most;
}
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

  it('draws each Exile status mark from the kit geometry, with the fill moved inside the ring, in rows and in detail', async () => {
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
      const kit = drawing(kitSvg(status));
      const drawn = drawing(svg);
      const fill = INSIDE_FILL[status];
      if (!fill) {
        // Hollow statuses are the kit's drawing exactly.
        expect(drawn, status).toEqual(kit);
        return;
      }
      // Owner, Live 2 (2026-11-02): the status colour fills only the inside.
      // The kit's ring keeps its stroke, width, dash and geometry, unfilled...
      const kitRing = kit.find((p) => p.d === RING)!;
      expect(kitRing.fill, `${status} kit ring fill`).toBe(fill);
      const rings = drawn.filter((p) => p.d === RING);
      expect(rings, status).toHaveLength(1);
      expect(rings[0], status).toEqual({ ...kitRing, fill: 'none' });
      // ...and the kit's fill colour lies on one inner diamond wholly inside
      // the ring's inner edge (radius 10 less the half stroke, about 9), with
      // a gap of the well between them (radius 8 at most).
      const filled = drawn.filter((p) => p.fill === fill);
      expect(filled, status).toHaveLength(1);
      expect(radius(filled[0].d!), `${status} inner fill`).toBeLessThanOrEqual(8);
      // The kit's other details stay, inside that fill.
      for (const detail of kit.filter((p) => p.d !== RING)) {
        const same = drawn.find((p) => p.d === detail.d && p.stroke === detail.stroke && p.fill === detail.fill);
        if (same) continue;
        // In progress's upper-facet highlight is moved inside the inner fill, its paint unchanged.
        const moved = drawn.find((p) => p !== filled[0] && p.fill === detail.fill && p.opacity === detail.opacity && p.d !== RING);
        expect(moved, `${status} detail ${detail.d}`).toBeDefined();
        expect(radius(moved!.d!), `${status} detail inside the fill`).toBeLessThanOrEqual(radius(filled[0].d!));
      }
      expect(drawn.length, status).toBe(kit.length + 1);
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

  it('draws the neutral frame as the kit nine-slice: 224 per side, 48 px wide, flush outsets, no centre', async () => {
    const frame = await readText('./components/MaterialFrame.svelte');
    expect(frame).toMatch(/border-image-slice:\s*224;/);
    expect(frame).not.toMatch(/border-image-slice:[^;]*fill/);
    expect(frame).toMatch(/border-image-repeat:\s*stretch;/);
    // A constant size on screen: divided by the zoom the scale setting applies.
    expect(frame).toMatch(/--frame-zoom:\s*var\(--page-zoom, var\(--preview-scale, 1\)\);/);
    expect(frame).toMatch(/border-image-width:\s*calc\(48px \/ var\(--frame-zoom\)\);/);
    // Flush to the window (owner, 2026-11-01): top, right, bottom, left just
    // under the PNG's own padding at 48 px (9 / 8.4 / 13.7 / 9 px), the same
    // in the overlay and the ordinary Settings window.
    expect(frame).toMatch(
      /border-image-outset:\s*calc\(9px \/ var\(--frame-zoom\)\) calc\(8px \/ var\(--frame-zoom\)\) calc\(13\.5px \/ var\(--frame-zoom\)\) calc\(8\.5px \/ var\(--frame-zoom\)\);/,
    );
    expect(frame).not.toMatch(/plaque/);
    expect(frame).toMatch(/pointer-events:\s*none;/);
    // Exile alone drops the overlay's transparent margin; the base rule keeps it for Claude.
    const approved = await readText('./approved.css');
    expect(approved).toMatch(/:root\[data-theme='exile'\] \.overlay:not\(\[data-frame='collapsed'\]\) \{ padding: 0; \}/);

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

  it('hides the scrollbar in Exile play only, keeping edit and the 8 px kit scrollbar', async () => {
    // Owner, Live 2 (2026-11-02): play is click-through, so its scrollbar
    // cannot be used. The scrollers still scroll (overflow stays auto); only
    // the bar goes, and only in play.
    const css = await readText('./approved.css');
    const rules = [...css.matchAll(/([^{}]+)\{([^{}]*)\}/g)].map((m) => ({ selector: m[1].replace(/\/\*[\s\S]*?\*\//g, '').trim(), body: m[2] }));
    const hiding = rules.filter((r) => /scrollbar-width:\s*none/.test(r.body));
    expect(hiding.length).toBeGreaterThan(0);
    for (const rule of hiding) {
      for (const selector of rule.selector.split(',').map((s) => s.trim())) {
        expect(selector, 'a rule that hides a scrollbar').toMatch(/^:root\[data-theme='exile'\] \.overlay\[data-mode='play'\] /);
        expect(selector).not.toMatch(/edit/);
      }
      expect(rule.body).not.toMatch(/overflow/);
    }
    // It reaches every scroller of the panel: the body, whole-content scrolling, the status line.
    const scoped = hiding.flatMap((r) => r.selector.split(',').map((s) => s.trim()));
    expect(scoped).toContain(":root[data-theme='exile'] .overlay[data-mode='play'] .panel *");
    // Edit, the overlay Settings frame, the picker and the ordinary Settings window keep the kit's 8 px bar.
    expect(css).toMatch(/:root\[data-theme='exile'\] ::-webkit-scrollbar \{ width: 8px; height: 8px; \}/);
    expect(css).toMatch(/:root\[data-theme='exile'\] ::-webkit-scrollbar-thumb \{[^}]*background: #64727C;/);
    expect(css).not.toMatch(/data-mode='edit'[^{]*\{[^}]*scrollbar-width:\s*none/);
    // The scrollers themselves still scroll in play.
    expect(css).toMatch(/:root\[data-theme='exile'\] \.panel\[data-scroll='whole'\] \.panel-content \{[^}]*overflow-y: auto;/);
  });

  it('groups Next up into labelled worth-group sections with a section-break between them, and leaves claude flat', async () => {
    // Owner, Live 3 (2026-11-02): which quests are worth it and which
    // optional must read at a glance. The kit preview's order: the diamond
    // divider breaks sections, a fine steel line separates quests inside one.
    const view = sectionsView(fixtureView('panel-next-up', 'play'), fixtureView('panel-campaign'));
    view.settings.theme = 'exile';
    const bridge = fakeBridge(view);
    rendered = await renderApp(bridge);
    const sections = [...rendered.root.querySelectorAll('.body > section.worth-group')];
    expect(sections.map((s) => s.getAttribute('data-worth'))).toEqual(['yes', 'optional', 'no']);
    const ids = new Set<string>();
    for (const section of sections) {
      const worth = section.getAttribute('data-worth') as Worth;
      const id = section.getAttribute('aria-labelledby')!;
      expect(id, worth).toBeTruthy();
      ids.add(id);
      const heading = rendered.root.querySelector(`[id="${id}"]`)!;
      expect(heading.classList.contains('group-label'), worth).toBe(true);
      expect(section.contains(heading), worth).toBe(true);
      expect(heading.textContent?.trim(), worth).toBe(WORTH_LABEL[worth]);
      // The group's own worth icon before the label, decoration only.
      const icon = heading.querySelector(':scope > .icon')!;
      expect(icon.getAttribute('data-icon'), worth).toBe(WORTH_ICON[worth]);
      expect(icon.getAttribute('aria-hidden'), worth).toBe('true');
      expect(icon.compareDocumentPosition(heading.querySelector(':scope > span:not(.icon)')!) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
      // Two quests each; the divider under the first only, never under a section's last quest.
      const rows = [...section.querySelectorAll(':scope > .row[data-quest]')];
      expect(rows, worth).toHaveLength(2);
      expect(rows[0].querySelectorAll(':scope > .quest-divider'), worth).toHaveLength(1);
      expect(rows[rows.length - 1].querySelector('.quest-divider'), `${worth} last row`).toBeNull();
    }
    expect(ids.size).toBe(3);
    // One section break, above the second and third headings only.
    expect(rendered.root.querySelectorAll('.section-break')).toHaveLength(2);
    expect(sections[0].querySelector('.section-break')).toBeNull();
    for (const section of sections.slice(1)) {
      const breaks = section.querySelectorAll(':scope > .section-break');
      expect(breaks).toHaveLength(1);
      expect(breaks[0].getAttribute('aria-hidden')).toBe('true');
      expect(breaks[0].compareDocumentPosition(section.querySelector('.group-label')!) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    }

    // The styles: the break is the kit divider as drawn (band, line, diamond);
    // the separator inside a section is the line alone, no diamond, no band;
    // the heading is kit gold.
    const css = await readText('./approved.css');
    const rule = (selector: RegExp) => [...css.matchAll(/([^{}]+)\{([^{}]*)\}/g)].filter((m) => selector.test(m[1])).map((m) => m[2]).join(' ');
    expect(rule(/\.section-break::before/)).toMatch(/transform: rotate\(45deg\)/);
    expect(rule(/\.section-break(?!::)/)).toMatch(/#353E44/);
    const separator = rule(/\.worth-group \.row\[data-quest\] > \.quest-divider(?!::)/);
    expect(separator).toMatch(/height: 1px/);
    expect(separator).not.toMatch(/#353E44/);
    expect(rule(/\.worth-group \.row\[data-quest\] > \.quest-divider::before/)).toMatch(/display: none/);
    expect(rule(/\.worth-group \.group-label(?!\s*>)/)).toMatch(/color: var\(--color-worth-gold\)/);
    expect(bridge.calls).toEqual([]);

    // Claude keeps today's flat list: no sections, no breaks, the same labels.
    rendered.destroy();
    const claude = sectionsView(fixtureView('panel-next-up', 'play'), fixtureView('panel-campaign'));
    claude.settings.theme = 'claude';
    rendered = await renderApp(fakeBridge(claude));
    expect(rendered.root.querySelector('.worth-group, section, .section-break, .quest-divider')).toBeNull();
    expect([...rendered.root.querySelectorAll('.body > p.group-label')].map((p) => p.textContent)).toEqual(['Worth it', 'Optional', 'Not worth it']);
  });
});
