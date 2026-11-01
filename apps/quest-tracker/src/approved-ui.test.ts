import { afterEach, describe, expect, it } from 'vitest';
import { FRAMES, QUEST_STATUSES } from './ipc';
import { STATUS_ICON, STATUS_LABEL } from './labels';
import EXILE from '../theme/exile.json';
import { CONTROL_SELECTOR, fakeBridge, fixtureView, readText, renderApp, type Rendered } from './test-support';

let rendered: Rendered | null = null;
afterEach(() => { rendered?.destroy(); rendered = null; });

describe('approved material adoption', () => {
  it('uses the compact tracker title, truthful act and visible character context', async () => {
    const view = fixtureView('panel-next-up', 'play');
    view.character!.name = 'Steel_Probe';
    view.acts[0].act = 4;
    view.settings.hotkey = 'Ctrl+Shift+J';
    const bridge = fakeBridge(view);
    rendered = await renderApp(bridge);
    expect(rendered.root.querySelector('.tracker-title')?.textContent).toBe('Quest Tracker');
    expect(rendered.root.querySelector('.tracker-act')?.textContent).toBe('Act IV');
    expect(rendered.root.querySelector('.character-context')?.textContent).toContain('Steel_Probe');
    expect(rendered.root.querySelector('.character-context')?.textContent).toContain(String(view.character!.level));
    expect(rendered.root.querySelector('.play-footer')?.textContent).toContain('Ctrl+Shift+J');
    expect(rendered.root.querySelectorAll(CONTROL_SELECTOR)).toHaveLength(0);
    expect(rendered.root.querySelector('.group-label')?.textContent).toBeTruthy();
    expect(rendered.root.querySelector('.rewards')?.textContent).toBeTruthy();
    bridge.pushView({ ...view, acts: [], next_up: [] });
    expect(rendered.root.querySelector('.tracker-act')).toBeNull();
    bridge.pushView({ ...view, acts: [view.acts[0], { ...view.acts[0], act: 5 }], next_up: [] });
    expect(rendered.root.querySelector('.tracker-act')).toBeNull();
    bridge.pushView({ ...view, next_up: [] });
    expect(rendered.root.querySelector('.tracker-act')?.textContent).toBe('Act IV');
    expect(bridge.calls).toEqual([]);
  });

  it('uses only the approved steel runtime surround', async () => {
    const frame = await readText('./components/MaterialFrame.svelte');
    expect(frame).toContain('frame-neutral-steel.png');
    expect(frame).not.toContain('tracker-steel-frame.png');
    expect(frame).not.toContain('journal-frame.png');
  });

  it('preserves all six Claude status symbols and current objective data', async () => {
    const view = fixtureView('panel-next-up', 'play');
    view.settings.theme = 'claude';
    const base = view.next_up[0];
    view.next_up = QUEST_STATUSES.map((status, i) => ({ ...base, id: `status-${i}`, status,
      current_step: status === 'in_progress' ? 'Current objective probe' : null,
      current_tip: status === 'in_progress' ? 'Current tip probe' : null }));
    const bridge = fakeBridge(view);
    rendered = await renderApp(bridge);
    for (const row of view.next_up) {
      const shown = rendered.root.querySelector(`[data-quest="${row.id}"]`)!;
      expect(shown.querySelector('.status-mark .icon')?.getAttribute('data-icon')).toBe(STATUS_ICON[row.status]);
      expect(shown.querySelector('.status-mark .icon')?.getAttribute('aria-label')).toBe(STATUS_LABEL[row.status]);
      expect(shown.querySelector('.step')?.textContent ?? null).toBe(row.current_step);
      expect(shown.querySelector('.tip')?.textContent ?? null).toBe(row.current_tip);
    }
    expect(bridge.calls).toEqual([]);
  });

  it('uses labelled diamond treatments for all six Exile statuses in rows and detail', async () => {
    const view = fixtureView('panel-next-up', 'play');
    view.settings.theme = 'exile';
    const base = view.next_up[0];
    view.next_up = QUEST_STATUSES.map((status, i) => ({ ...base, id: `diamond-${i}`, status }));
    const bridge = fakeBridge(view);
    rendered = await renderApp(bridge);
    function checkMark(mark: Element, status: typeof QUEST_STATUSES[number]): void {
      expect(mark.getAttribute('role')).toBe('img');
      expect(mark.getAttribute('aria-label')).toBe(STATUS_LABEL[status]);
      expect(mark.getAttribute('title')).toBe(STATUS_LABEL[status]);
      // The kit diamond is drawn inline and announces nothing of its own
      // (its geometry is pinned in steel-kit.test.ts); no status icon inside.
      expect(mark.querySelector('svg.status-diamond')?.getAttribute('aria-hidden')).toBe('true');
      expect(mark.querySelectorAll('svg')).toHaveLength(1);
      expect(mark.querySelector('.icon, .glyph')).toBeNull();
      expect(mark.querySelector('[aria-label]')).toBeNull();
    }
    for (const row of view.next_up) {
      checkMark(rendered.root.querySelector(`[data-quest="${row.id}"] > .status-mark`)!, row.status);
    }
    const detail = fixtureView('quest-detail', 'play');
    detail.settings.theme = 'exile';
    for (const status of QUEST_STATUSES) {
      detail.detail!.row.status = status;
      bridge.pushView(detail);
      checkMark(rendered.root.querySelector('.detail .head > .status-mark')!, status);
    }
    bridge.pushView({ ...view, settings: { ...view.settings, theme: 'claude' } });
    for (const row of view.next_up) {
      const mark = rendered.root.querySelector(`[data-quest="${row.id}"] > .status-mark`)!;
      expect(mark.querySelector('.status-diamond')).toBeNull();
      expect(mark.querySelector('.icon')?.getAttribute('data-icon')).toBe(STATUS_ICON[row.status]);
      expect(mark.querySelector('.icon')?.getAttribute('aria-label')).toBe(STATUS_LABEL[row.status]);
    }
    expect(bridge.calls).toEqual([]);
  });
  it('keeps the selected material across every mode and expanded frame', async () => {
    for (const theme of ['exile', 'claude'] as const) for (const mode of ['play', 'edit'] as const) for (const frame of FRAMES) {
      const view = fixtureView(frame, mode);
      view.settings.theme = theme;
      const bridge = fakeBridge(view);
      rendered = await renderApp(bridge);
      if (frame === 'collapsed') {
        expect(rendered.root.querySelector('.panel')).toBeNull();
      } else {
        expect(rendered.root.querySelector('.panel')?.getAttribute('data-material')).toBe(
          theme === 'exile' ? 'journal' : 'claude',
        );
        expect(Boolean(rendered.root.querySelector('.material-art'))).toBe(theme === 'exile');
        expect(rendered.root.querySelector('.journal-plaque, .journal-crest')).toBeNull();
        expect(rendered.root.querySelector('.header h1')).not.toBeNull();
      }
      expect(document.documentElement.dataset.theme).toBe(theme);
      expect(bridge.calls).toEqual([]);
      if (mode === 'play') expect(rendered.root.querySelectorAll(CONTROL_SELECTOR)).toHaveLength(0);
      rendered.destroy(); rendered = null;
    }
  });

  it('changes mode controls without changing the selected theme or requesting a theme patch', async () => {
    for (const theme of ['exile', 'claude'] as const) {
      const view = fixtureView('settings', 'edit');
      view.settings.theme = theme;
      const bridge = fakeBridge(view);
      rendered = await renderApp(bridge);
      (rendered.root.querySelector('.back-to-play') as HTMLButtonElement).click();
      expect(bridge.calls).toEqual([{ name: 'set_mode', args: { mode: 'play' } }]);
      bridge.pushView({ ...view, mode: 'play', frame: 'panel-next-up' });
      expect(document.documentElement.dataset.theme).toBe(theme);
      expect(rendered.root.querySelector('.panel')?.getAttribute('data-material')).toBe(theme === 'exile' ? 'journal' : 'claude');
      expect(Boolean(rendered.root.querySelector('.material-art'))).toBe(theme === 'exile');
      expect(rendered.root.querySelectorAll(CONTROL_SELECTOR)).toHaveLength(0);
      bridge.pushView(view);
      expect((rendered.root.querySelector('.theme-choice select') as HTMLSelectElement).value).toBe(theme);
      expect(bridge.calls.some((call) => call.name === 'update_settings')).toBe(false);
      rendered.destroy(); rendered = null;
    }
  });

  it('keeps text and commands across frame changes without making ornament interactive', async () => {
    const view = fixtureView('panel-campaign', 'edit');
    const long = 'The long invented campaign name '.repeat(8);
    view.character!.name = long;
    view.acts[0].quests[0].name = long;
    view.status = { kind: 'error', text: `Recovery: ${long}` };
    const bridge = fakeBridge(view);
    rendered = await renderApp(bridge);
    expect(rendered.root.textContent).toContain(long);
    const row = view.acts[0].quests[0];
    (rendered.root.querySelector(`[data-quest="${row.id}"]`) as HTMLButtonElement).click();
    expect(bridge.calls.at(-1)).toEqual({ name: 'open_detail', args: { questId: row.id } });
    for (const art of rendered.root.querySelectorAll('.material-art')) {
      expect(art.getAttribute('aria-hidden')).toBe('true');
      expect(art.querySelector(CONTROL_SELECTOR)).toBeNull();
    }
    expect(rendered.root.querySelector('.material-art')).not.toBeNull();
    bridge.pushView({ ...view, frame: 'panel-next-up' });
    (rendered.root.querySelector('[aria-label="Settings"]') as HTMLButtonElement).click();
    expect(bridge.calls.at(-1)).toEqual({ name: 'open_frame', args: { frame: 'settings' } });
    expect(rendered.root.textContent).toContain(long);
  });

  it('keeps JSON and CSS token values synchronized, including numerical units', async () => {
    const css = await readText('../theme/exile.css');
    const vars = new Map([...css.matchAll(/(--[\w-]+):\s*([^;]+);/g)].map((m) => [m[1], m[2].trim()]));
    const unitless = /^(opacity\.|font\.weight|motion\.confirm-ms$)/;
    for (const [key, value] of Object.entries(EXILE)) {
      let expected: string;
      if (typeof value === 'number') expected = `${value}${key === 'motion.confirm-ms' ? 'ms' : unitless.test(key) ? '' : 'px'}`;
      else if (key.startsWith('font.file')) expected = `url("${value}")`;
      else if (key.startsWith('font.family')) expected = `"${value}", ${['font.family-caps', 'font.family-quest'].includes(key) ? 'serif' : 'sans-serif'}`;
      else expected = value;
      expect(vars.get(`--${key.replace('.', '-')}`), key).toBe(expected);
    }
  });

  it('keeps a fixed Settings drag surface while excluding controls and passive play', async () => {
    const view = fixtureView('settings', 'edit');
    const bridge = fakeBridge(view);
    rendered = await renderApp(bridge);
    const press = (element: Element, button = 0) => element.dispatchEvent(new MouseEvent('pointerdown', { bubbles: true, button }));
    const grab = rendered.root.querySelector('.frame-drag-region')!;
    expect(grab.closest('.panel-content')).toBeNull();
    press(grab);
    expect(bridge.drags).toBe(1);
    press(rendered.root.querySelector('.header')!);
    expect(bridge.drags).toBe(2);
    press(grab, 2);
    for (const control of rendered.root.querySelectorAll('button,input,select')) press(control);
    expect(bridge.drags).toBe(2);
    expect(bridge.calls).toEqual([]);
    bridge.pushView({ ...view, mode: 'play' });
    expect(rendered.root.querySelector('.frame-drag-region')).toBeNull();
    press(rendered.root.querySelector('.header')!);
    expect(bridge.drags).toBe(2);
    bridge.pushView({ ...view, settings: { ...view.settings, theme: 'claude' } });
    expect(rendered.root.querySelector('.frame-drag-region')).toBeNull();
    press(rendered.root.querySelector('.header .spacer')!);
    expect(bridge.drags).toBe(3);
  });

  it('uses authoritative theme views and preserves the original Claude surface and command payloads', async () => {
    const view = fixtureView('settings', 'edit');
    const bridge = fakeBridge(view);
    rendered = await renderApp(bridge);
    const select = rendered.root.querySelector('.theme-choice select') as HTMLSelectElement;
    expect([...select.options].map((o) => o.text)).toEqual(['Exile', 'Claude']);
    expect(select.value).toBe('exile');
    select.value = 'claude';
    select.dispatchEvent(new Event('change', { bubbles: true }));
    expect(bridge.calls.at(-1)).toEqual({ name: 'update_settings', args: { patch: { theme: 'claude' } } });
    expect(rendered.root.querySelector('.overlay')?.getAttribute('data-theme')).toBe('exile');
    const claude = structuredClone(view);
    claude.settings.theme = 'claude';
    bridge.pushView(claude);
    expect(rendered.root.querySelector('.overlay')?.getAttribute('data-theme')).toBe('claude');
    expect(document.documentElement.dataset.theme).toBe('claude');
    expect(rendered.root.querySelector('.material-art')).toBeNull();
    expect(rendered.root.querySelector('.journal-plaque')).toBeNull();
    expect(rendered.root.querySelector('.panel')?.getAttribute('data-material')).toBe('claude');
    expect(rendered.root.querySelectorAll('.settings input')).toHaveLength(8);
    bridge.pushView(view);
    expect(document.documentElement.dataset.theme).toBe('exile');
    expect(rendered.root.querySelector('.material-art')).not.toBeNull();
  });

  it('keeps original material source art and the licensed display face locally', async () => {
    const art = import.meta.glob('../theme/materials/*.svg', { query: '?raw', import: 'default', eager: true }) as Record<string, string>;
    for (const svg of Object.values(art)) {
      expect(svg).toMatch(/^<svg/);
      expect(svg).not.toMatch(/<script|<image\b|<foreignObject|\son[a-z]+=|(?:xlink:)?href\s*=/i);
      expect(svg.replace('http://www.w3.org/2000/svg', '')).not.toMatch(/https?:|url\((?!#)/i);
    }
    const fs = await import(/* @vite-ignore */ 'node:fs' as string) as { readFileSync(path: URL): Uint8Array };
    const rasters = {
      'journal-frame.png': '0745d20818040cf3b5796acab4d54de981b38ddf53b47a611437992dae57a3c2',
      'lantern-astrolabe-crest.png': '1ff2e55ca20f94f2614b929543f7148f3a997261e1d51660974ebcae3ca443d6',
      'charcoal-stone-well.png': '72cdd6a41d7f5f7ea89dc76de917407c3533f12df8733cddc4e41bee6f7cb225',
      'tracker-steel-frame.png': '438cc3a542d639ef948243f45d6a6870970bc7d5cf3bc18779e44c244beda796',
      // The steel/masonry kit's two finished rasters, copied unchanged.
      'frame-neutral-steel.png': '5a716ea986840137e56890dba462be91fb17c119c4584c88d8b2002579249413',
      'masonry-charcoal.png': 'b0618d6f87b90e115c8f6a7521ab14353be71a0d7f91daa6cfe501d08d254114',
    };
    const crypto = await import(/* @vite-ignore */ 'node:crypto' as string) as { createHash(name: string): { update(data: Uint8Array): { digest(encoding: string): string } } };
    expect(Object.keys(import.meta.glob('../theme/materials/raster/*.png'))).toHaveLength(6);
    for (const [name, hash] of Object.entries(rasters)) {
      const path = `../theme/materials/raster/${name}`;
      const png = fs.readFileSync(new URL(path, import.meta.url));
      expect([...png.slice(0, 8)], name).toEqual([137, 80, 78, 71, 13, 10, 26, 10]);
      // Colour type 2 (RGB) for the two masonry wells, 6 (RGBA) for the framed art.
      expect(png[25], name).toBe(name === 'charcoal-stone-well.png' || name === 'masonry-charcoal.png' ? 2 : 6);
      expect(crypto.createHash('sha256').update(png).digest('hex'), name).toBe(hash);
    }
    const fontPath = '../theme/fonts/Cinzel-SemiBold.woff2';
    const bytes = fs.readFileSync(new URL(fontPath, import.meta.url));
    expect(String.fromCharCode(...bytes.slice(0, 4))).toBe('wOF2');
    const license = await readText('../theme/fonts/Cinzel-OFL.txt');
    expect(license).toContain('The Cinzel Project Authors');
    expect(license).toContain('SIL OPEN FONT LICENSE Version 1.1');
  });

  it('changes only the new-store native default and keeps Tauri and shell in agreement', async () => {
    const shell = await readText('../crates/qt-app/src/app.rs');
    const config = JSON.parse(await readText('../src-tauri/tauri.conf.json'));
    expect(shell.match(/DEFAULT_WINDOW_SIZE:[^;]+/)?.[0]).toMatch(/= \(364\.0, 384\.0\)/);
    expect(shell.match(/MIN_WINDOW_SIZE:[^;]+/)?.[0]).toMatch(/= \(320\.0, 240\.0\)/);
    expect([config.app.windows[0].width, config.app.windows[0].height]).toEqual([364, 384]);
  });
});
