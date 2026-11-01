import { afterEach, describe, expect, it } from 'vitest';
import { FRAMES } from './ipc';
import EXILE from '../theme/exile.json';
import { CONTROL_SELECTOR, fakeBridge, fixtureView, readText, renderApp, type Rendered } from './test-support';

let rendered: Rendered | null = null;
afterEach(() => { rendered?.destroy(); rendered = null; });

describe('approved material adoption', () => {
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
    press(rendered.root.querySelector('.journal-plaque')!);
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

  it('bundles original material art and the licensed display face locally', async () => {
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
    };
    const crypto = await import(/* @vite-ignore */ 'node:crypto' as string) as { createHash(name: string): { update(data: Uint8Array): { digest(encoding: string): string } } };
    expect(Object.keys(import.meta.glob('../theme/materials/raster/*.png'))).toHaveLength(3);
    for (const [name, hash] of Object.entries(rasters)) {
      const path = `../theme/materials/raster/${name}`;
      const png = fs.readFileSync(new URL(path, import.meta.url));
      expect([...png.slice(0, 8)], name).toEqual([137, 80, 78, 71, 13, 10, 26, 10]);
      expect(png[25], name).toBe(name === 'charcoal-stone-well.png' ? 2 : 6);
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
