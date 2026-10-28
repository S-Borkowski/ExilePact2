import { flushSync } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import appSource from './App.svelte?raw';
import bridgeSource from './bridge.ts?raw';
import panelSource from './components/Panel.svelte?raw';
import { RESIZE_DIRECTIONS, type ResizeDirection } from './bridge';
import type { View } from './ipc';
import { previewBridge } from './preview';
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
    // The overlay is the window's height, its 12 px margin inside it (in
    // the preview, divided by the CSS zoom that stands in for the webview's).
    const overlay = rule(appSource, '.overlay');
    expect(overlay).toMatch(/height:\s*calc\(100vh \/ var\(--preview-scale, 1\)\)/);
    expect(overlay).toMatch(/box-sizing:\s*border-box/);
    expect(overlay).toMatch(/padding:\s*12px/);
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
});
