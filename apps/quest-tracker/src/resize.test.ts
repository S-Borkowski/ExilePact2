import { flushSync } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import appSource from './App.svelte?raw';
import bridgeSource from './bridge.ts?raw';
import panelSource from './components/Panel.svelte?raw';
import { RESIZE_DIRECTIONS, type ResizeDirection } from './bridge';
import type { View } from './ipc';
import { PREVIEW_SCALES, previewBridge } from './preview';
import { type Rendered, fakeBridge, fixtureView, renderApp } from './test-support';

// 0.1.1: the scale zooms the content and the player sizes the window. In
// edit mode the panel's four edges and four corners are resize handles that
// start the window's own resize drag (the bridge's `startResize`); play
// mode and the collapsed badge have none, so the window stays click-through.
// The panel fills the window, minus the margin that holds its shadow, and
// its body scrolls. Views are cloned fixtures.

let rendered: Rendered | null = null;

afterEach(() => {
  rendered?.destroy();
  rendered = null;
});

async function render(view: View) {
  const bridge = fakeBridge(view);
  rendered = await renderApp(bridge);
  return { root: rendered.root, bridge };
}

function close(): void {
  rendered?.destroy();
  rendered = null;
}

function press(target: Element, button = 0): void {
  target.dispatchEvent(new MouseEvent('pointerdown', { bubbles: true, cancelable: true, button }));
  flushSync();
}

const handles = (root: Element) => [...root.querySelectorAll<HTMLElement>('[data-resize]')];

/** Each handle's resize cursor, by direction. */
const CURSOR: Record<ResizeDirection, string> = {
  North: 'ns-resize',
  South: 'ns-resize',
  East: 'ew-resize',
  West: 'ew-resize',
  NorthWest: 'nwse-resize',
  SouthEast: 'nwse-resize',
  NorthEast: 'nesw-resize',
  SouthWest: 'nesw-resize',
};

/** The body of a CSS rule in a component's `<style>`, by its exact selector. */
function rule(source: string, selector: string): string {
  const style = source.slice(source.indexOf('<style>'));
  const at = style.indexOf(`${selector} {`);
  expect(at, `no ${selector} rule`).toBeGreaterThanOrEqual(0);
  return style.slice(at, style.indexOf('}', at));
}

/** A declaration's value in a CSS rule body, e.g. `padding`. */
function declared(body: string, property: string): string {
  const m = new RegExp(`(?:^|[;{\\s])${property}:\\s*([^;]+);`).exec(body);
  expect(m, `no ${property}`).not.toBeNull();
  return m![1].trim();
}

/** A custom property as `el` inherits it from its own or an ancestor's inline style. */
function inherited(el: Element | null, name: string): string {
  for (let e = el; e; e = e.parentElement) {
    const v = (e as HTMLElement).style?.getPropertyValue(name).trim();
    if (v) return v;
  }
  return '';
}

/**
 * Evaluates the few CSS value forms the overlay uses (`12px`, `100vh`, a
 * plain number, `var(--x, fallback)`, `calc(a / b)` and `calc(a * b)`) to
 * CSS px, `vh` being `vhPx` CSS px.
 */
function cssLength(value: string, el: Element, vhPx: number): number {
  const v = value.trim();
  const calc = /^calc\((.*)\)$/.exec(v);
  if (calc) {
    const inner = calc[1];
    let depth = 0;
    for (let i = 0; i < inner.length; i++) {
      const c = inner[i];
      if (c === '(') depth++;
      else if (c === ')') depth--;
      else if (depth === 0 && (c === '/' || c === '*')) {
        const a = cssLength(inner.slice(0, i), el, vhPx);
        const b = cssLength(inner.slice(i + 1), el, vhPx);
        return c === '/' ? a / b : a * b;
      }
    }
    return cssLength(inner, el, vhPx);
  }
  const ref = /^var\((--[\w-]+)(?:,\s*(.*))?\)$/.exec(v);
  if (ref) {
    const set = inherited(el, ref[1]);
    if (set) return cssLength(set, el, vhPx);
    expect(ref[2], `${ref[1]} unset, no fallback`).toBeDefined();
    return cssLength(ref[2], el, vhPx);
  }
  const unit = /^(-?[\d.]+)(px|vh)?$/.exec(v);
  expect(unit, `unsupported CSS value ${v}`).not.toBeNull();
  const n = Number(unit![1]);
  return unit![2] === 'vh' ? n * vhPx : n;
}

/**
 * Inside the app, a webview zoom `z` as measured in the built app (Live 1's
 * fault, re-measured for scale-fill): the window keeps its size, which the
 * page reads as `outerWidth`/`outerHeight`, while the viewport shrinks to
 * the window over the zoom (`innerWidth`, `innerHeight` and `100vh`), and a
 * resize event fires.
 */
function zoomTo(z: number, width: number, height: number): void {
  const set = (name: string, value: number) => Object.defineProperty(window, name, { configurable: true, value });
  set('outerWidth', width);
  set('outerHeight', height);
  set('innerWidth', width / z);
  set('innerHeight', height / z);
  window.dispatchEvent(new Event('resize'));
  flushSync();
}

/**
 * Where the panel draws, in window (or page) pixels, by the box chain
 * `panel_fills_the_window` pins: the overlay's height and padding from its
 * rule, evaluated as the overlay inherits its variables, and the panel
 * filling the overlay's content box. `vhPx` is one `vh` in CSS px, `zoom`
 * how many window pixels one CSS px draws.
 */
function drawnPanel(root: Element, page: { width: number; height: number }, vhPx: number, zoom: number) {
  const overlay = root.querySelector('.overlay')!;
  const body = rule(appSource, '.overlay');
  const padding = cssLength(declared(body, 'padding'), overlay, vhPx);
  const height = cssLength(declared(body, 'height'), overlay, vhPx);
  // A block's auto width is its containing block's: the page's, in CSS px.
  const width = page.width / zoom;
  return { width: (width - 2 * padding) * zoom, height: (height - 2 * padding) * zoom, margin: padding * zoom };
}

const JSDOM_SIZE = { innerWidth: window.innerWidth, innerHeight: window.innerHeight, outerWidth: window.outerWidth, outerHeight: window.outerHeight };

function resetZoom(): void {
  for (const [name, value] of Object.entries(JSDOM_SIZE)) Object.defineProperty(window, name, { configurable: true, value });
  const root = document.documentElement;
  delete root.dataset.previewScale;
  root.style.removeProperty('--preview-scale');
  root.style.removeProperty('--page-zoom');
}

describe('resizing the window by its edges', () => {
  it('edge_press_starts_a_resize_in_edit_mode', async () => {
    const { root, bridge } = await render(fixtureView('panel-campaign', 'edit'));
    const found = handles(root);
    // The eight handles, each once, on the panel (not in its scrolled body).
    expect(found.map((h) => h.dataset.resize).sort()).toEqual([...RESIZE_DIRECTIONS].sort());
    for (const h of found) {
      expect(h.closest('.panel-shadow'), h.dataset.resize).not.toBeNull();
      expect(h.closest('.body'), h.dataset.resize).toBeNull();
      // A pointer affordance only: hidden from assistive technology, no tab stop.
      expect(h.getAttribute('aria-hidden')).toBe('true');
      expect(h.hasAttribute('tabindex')).toBe(false);
    }

    // A left press on each handle starts that direction's resize, and no
    // window drag and no command.
    for (const h of found) press(h);
    expect(bridge.resizes).toEqual(found.map((h) => h.dataset.resize));
    expect(bridge.drags).toBe(0);
    expect(bridge.calls).toEqual([]);
    // Another mouse button starts none.
    press(found[0], 2);
    press(found[3], 1);
    expect(bridge.resizes).toHaveLength(8);
    // A press elsewhere on the panel starts none either.
    press(root.querySelector('.body')!);
    press(root.querySelector('header.header .spacer')!);
    expect(bridge.resizes).toHaveLength(8);
    expect(bridge.drags).toBe(1);

    // Each handle shows its matching resize cursor.
    for (const direction of RESIZE_DIRECTIONS) {
      expect(panelSource).toMatch(new RegExp(`\\[data-resize='${direction}'\\][^{]*\\{[^}]*cursor:\\s*${CURSOR[direction]}`));
    }

    // The preview records the press as the test bridge does, and sends no
    // command.
    const preview = previewBridge('');
    expect(() => preview.startResize('SouthEast')).not.toThrow();
    expect(preview.resizes).toEqual(['SouthEast']);
    expect(preview.calls).toEqual([]);
    // Inside the app, the bridge asks the Tauri window to start resizing in
    // that direction, as the header's press starts `startDragging()`.
    expect(bridgeSource).toMatch(/getCurrentWindow\(\)\s*\.startResizeDragging\(direction\)/);
  });

  it('play_mode_has_no_resize_edges', async () => {
    // Baseline: the edit-mode panel has its eight.
    const edit = await render(fixtureView('panel-next-up', 'edit'));
    expect(handles(edit.root)).toHaveLength(8);
    close();

    // Play mode: none, on every frame the panel shows, and a press anywhere
    // starts no resize.
    for (const frame of ['panel-campaign', 'panel-next-up', 'quest-detail', 'settings', 'empty-no-log']) {
      const play = await render(fixtureView(frame, 'play'));
      expect(handles(play.root), frame).toEqual([]);
      press(play.root.querySelector('.panel')!);
      expect(play.bridge.resizes, frame).toEqual([]);
      close();
    }

    // The collapsed badge has none, in either mode.
    for (const mode of ['play', 'edit'] as const) {
      const badge = await render(fixtureView('collapsed', mode));
      expect(badge.root.querySelector('.badge'), mode).not.toBeNull();
      expect(handles(badge.root), mode).toEqual([]);
      close();
    }
  });

  it('panel_fills_the_window', async () => {
    // The overlay is the window's height (in the preview, divided by the CSS
    // zoom that stands in for the webview's), its 12 px margin inside it,
    // divided by the page's zoom so that it stays 12 window pixels.
    const overlay = rule(appSource, '.overlay');
    expect(overlay).toMatch(/height:\s*calc\(100vh \/ var\(--preview-scale, 1\)\)/);
    expect(overlay).toMatch(/box-sizing:\s*border-box/);
    expect(overlay).toMatch(/padding:\s*calc\(12px \/ var\(--page-zoom, 1\)\)/);
    // The panel takes the rest in both directions: no fixed width and no
    // height cap any more.
    const shadow = rule(panelSource, '.panel-shadow');
    expect(shadow).toMatch(/flex:\s*1 1 auto/);
    expect(shadow).toMatch(/align-self:\s*stretch/);
    expect(shadow).toMatch(/min-height:\s*0/);
    const panel = rule(panelSource, '.panel');
    expect(panel).toMatch(/width:\s*100%/);
    expect(panel).toMatch(/flex:\s*1 1 auto/);
    expect(panel).not.toMatch(/max-height/);
    expect(panel).not.toMatch(/--size-panel-width/);
    // Its body takes the height the header, status line and footer leave,
    // and scrolls.
    const body = rule(panelSource, '.body');
    expect(body).toMatch(/flex:\s*1 1 auto/);
    expect(body).toMatch(/min-height:\s*0/);
    expect(body).toMatch(/overflow-y:\s*auto/);

    // Rendered: the body holds the rows, between the header and the way
    // back to play.
    const { root } = await render(fixtureView('panel-campaign', 'edit'));
    const panelEl = root.querySelector('.panel')!;
    const kids = [...panelEl.children].map((c) => c.className.split(' ')[0]);
    expect(kids[0]).toBe('header');
    expect(kids.at(-1)).toBe('back-to-play');
    expect(panelEl.querySelector('.body section.act')).not.toBeNull();
  });

  it('panel_fills_the_window_at_scale_one', async () => {
    // Baseline: at zoom 1 the panel draws over the window minus the 12 px
    // margin, in both directions, for the default-sized window and Live 1's.
    try {
      const { root } = await render(fixtureView('panel-campaign', 'edit'));
      for (const [width, height] of [
        [600, 700],
        [818, 808],
      ]) {
        zoomTo(1, width, height);
        const drawn = drawnPanel(root, { width, height }, window.innerHeight / 100, 1);
        expect(drawn, `${width}x${height}`).toEqual({ width: width - 24, height: height - 24, margin: 12 });
      }
    } finally {
      resetZoom();
    }
  });

  it('panel_fills_the_window_at_every_scale', async () => {
    // Live 1: with the window kept at 818 x 808, the panel's frame grew as
    // the scale fell and shrank as it rose. Measured in the built app: the
    // webview zoom lays the overlay out over the whole viewport (the window
    // over the zoom), but its margin is in CSS px, which the zoom draws
    // larger, so the frame was the window minus 24 px times the zoom. The
    // margin now stays 12 window pixels: the frame is the same at every
    // scale and only the content zooms.
    const close6 = (n: number) => Math.round(n * 1e6) / 1e6;
    try {
      const { root } = await render(fixtureView('settings', 'edit'));
      for (const [width, height] of [
        [818, 808],
        [600, 700],
      ]) {
        for (const z of [0.75, 1.33, 1.5, 2, 1]) {
          zoomTo(z, width, height);
          const drawn = drawnPanel(root, { width, height }, window.innerHeight / 100, z);
          expect(
            { width: close6(drawn.width), height: close6(drawn.height), margin: close6(drawn.margin) },
            `${width}x${height} at ${z}`,
          ).toEqual({ width: width - 24, height: height - 24, margin: 12 });
        }
      }
    } finally {
      close();
      resetZoom();
    }

    // The browser preview stands in for the webview zoom with a CSS `zoom`
    // on the page, which leaves the viewport alone and draws every CSS
    // length, `vh` included, that many times larger: its panel fills the
    // page minus the same 12 px at each of its scales.
    try {
      for (const s of PREVIEW_SCALES) {
        const bridge = previewBridge(`?frame=panel-campaign&mode=edit&scale=${s}`, document.documentElement);
        rendered = await renderApp(bridge);
        const page = { width: window.innerWidth, height: window.innerHeight };
        const drawn = drawnPanel(rendered.root, page, page.height / 100, Number(s));
        expect(
          { width: close6(drawn.width), height: close6(drawn.height), margin: close6(drawn.margin) },
          `preview at ${s}`,
        ).toEqual({ width: page.width - 24, height: page.height - 24, margin: 12 });
        close();
        resetZoom();
      }
    } finally {
      resetZoom();
    }
  });
});
