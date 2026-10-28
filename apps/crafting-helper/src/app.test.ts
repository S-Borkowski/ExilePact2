// The main window (`index.html`): every view fixture renders, a grab shows
// its raw text and line count, a paste becomes `grab_text` (and a paste into
// a field does not), the Settings button names an update, the Tauri bridge
// subscribes before it asks, and the preview reads its fixture from the
// query.

import { afterEach, describe, expect, it, vi } from 'vitest';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { emit } from '@tauri-apps/api/event';
import { DEFAULT_PREVIEW_VIEW, VIEW_EVENT, previewBridge, previewViewName, tauriBridge } from './bridge';
import type { View } from './ipc';
import { FIXTURE_NAMES, type Rendered, button, click, fixtureView, paste, renderComponent, renderView, text } from './test-support';
import App from './App.svelte';

let mounted: Rendered[] = [];

async function render(view: View) {
  const r = await renderView(view);
  mounted.push(r);
  return r;
}

afterEach(() => {
  for (const r of mounted) r.destroy();
  mounted = [];
  vi.restoreAllMocks();
  clearMocks();
});

describe('the main window', () => {
  it('placeholder_shows_raw_text_and_line_count', async () => {
    // No grab: how to grab, with the current hotkey, and no text block.
    const empty = fixtureView('empty');
    const { root, bridge } = await render(empty);
    expect(root.querySelector('.app')?.getAttribute('data-state')).toBe('empty');
    expect(text(root.querySelector('h1'))).toBe('Crafting Helper');
    expect(text(root.querySelector('h2'))).toBe('Last grabbed item');
    expect(root.querySelector('pre')).toBeNull();
    const howto = text(root.querySelector('.howto'));
    expect(howto).toContain('Ctrl+Alt+C');
    expect(howto).toContain('Ctrl+V');
    expect(text(root.querySelector('kbd.hotkey'))).toBe('Ctrl+Alt+X');
    expect(text(root.querySelector('[role="status"]'))).toBe('');

    // A grab: its line count, where it came from, the raw text exactly as sent.
    const grabbed = fixtureView('grabbed-item');
    bridge.pushView(grabbed);
    expect(root.querySelector('.app')?.getAttribute('data-state')).toBe('grabbed');
    expect(root.querySelector('.howto')).toBeNull();
    expect(text(root.querySelector('.count'))).toBe('10 lines');
    expect(text(root.querySelector('.source'))).toBe('from the hotkey');
    const raw = root.querySelector('pre.raw')!;
    expect(raw.textContent).toBe(grabbed.grab!.text);
    expect(raw.textContent!.split('\n')).toHaveLength(grabbed.grab!.line_count);
    expect(text(root.querySelector('[role="status"]'))).toBe('Grabbed an item: 10 lines.');
    expect(root.querySelector('.status')?.getAttribute('data-kind')).toBe('info');

    // A paste that is not an item: shown all the same, with its error marked "!".
    const notItem = fixtureView('grabbed-not-item');
    bridge.pushView(notItem);
    expect(text(root.querySelector('.count'))).toBe('3 lines');
    expect(text(root.querySelector('.source'))).toBe('from a paste');
    expect(root.querySelector('pre.raw')!.textContent).toBe('a shopping list\nmilk\neggs');
    const status = root.querySelector('[role="status"]')!;
    expect(status.getAttribute('aria-live')).toBe('polite');
    expect(text(status)).toBe(`!${notItem.status!.text}`);
    expect(status.querySelector('.mark')?.getAttribute('aria-hidden')).toBe('true');

    // One line is "1 line".
    bridge.pushView({ ...notItem, grab: { ...notItem.grab!, text: 'milk', line_count: 1 } });
    expect(text(root.querySelector('.count'))).toBe('1 line');

    // A refused grab keeps the last one and says why; a taken hotkey says so with no grab.
    const refused = await render(fixtureView('grab-refused'));
    expect(refused.root.querySelector('pre.raw')!.textContent).toBe(fixtureView('grab-refused').grab!.text);
    expect(text(refused.root.querySelector('[role="status"]'))).toBe('!Not grabbed: the clipboard is empty.');
    const taken = await render(fixtureView('hotkey-taken'));
    expect(taken.root.querySelector('pre')).toBeNull();
    expect(text(taken.root.querySelector('[role="status"]'))).toContain('taken by another program');

    // Every fixture renders, and none sends a command on its own.
    for (const name of FIXTURE_NAMES) {
      const r = await render(fixtureView(name));
      expect(r.root.querySelector('.app')?.getAttribute('data-state'), name).toBe(fixtureView(name).grab ? 'grabbed' : 'empty');
      expect(r.bridge.calls, name).toEqual([]);
    }
  });

  it('paste_sends_grab_text', async () => {
    const { root, bridge } = await render(fixtureView('empty'));
    const item = 'Item Class: Gloves\r\nRarity: Rare\r\n';

    // A paste anywhere in the window sends its text, as pasted: the app
    // normalises and validates it.
    const event = paste(root.querySelector('h2')!, item);
    expect(bridge.calls).toEqual([{ name: 'grab_text', args: { text: item } }]);
    expect(event.defaultPrevented).toBe(true);
    paste(document.body, 'second');
    expect(bridge.calls[1]).toEqual({ name: 'grab_text', args: { text: 'second' } });

    // A paste with no text still goes to the app, which refuses it in words.
    paste(document.body, null);
    paste(document.body, '');
    expect(bridge.calls.slice(2)).toEqual([
      { name: 'grab_text', args: { text: '' } },
      { name: 'grab_text', args: { text: '' } },
    ]);

    // Negative controls: a paste into an editable field is the field's.
    bridge.calls.length = 0;
    for (const html of ['<input type="text">', '<textarea></textarea>', '<div contenteditable="true"><span>x</span></div>']) {
      const host = document.createElement('div');
      host.innerHTML = html;
      root.querySelector('.app')!.appendChild(host);
      const field = host.querySelector('span') ?? host.firstElementChild!;
      const e = paste(field, item);
      expect(e.defaultPrevented, html).toBe(false);
      host.remove();
    }
    expect(bridge.calls).toEqual([]);

    // Nothing reads the clipboard but the paste event: no paste, no command.
    const readText = vi.fn();
    Object.defineProperty(navigator, 'clipboard', { value: { readText }, configurable: true });
    const fresh = await render(fixtureView('grabbed-item'));
    click(button(fresh.root, 'Settings'));
    window.dispatchEvent(new Event('focus'));
    expect(fresh.bridge.calls).toEqual([{ name: 'open_settings', args: {} }]);
    expect(readText).not.toHaveBeenCalled();
    delete (navigator as unknown as Record<string, unknown>).clipboard;

    // Once unmounted, the window no longer listens.
    const gone = await renderView(fixtureView('empty'));
    gone.destroy();
    paste(document.body, item);
    expect(gone.bridge.calls).toEqual([]);
  });

  it('settings_button_names_an_update', async () => {
    const { root, bridge } = await render(fixtureView('empty'));
    const settings = button(root, 'Settings')!;
    expect(settings.getAttribute('aria-label')).toBeNull();
    expect(root.querySelector('.update-mark')).toBeNull();
    click(settings);
    expect(bridge.calls).toEqual([{ name: 'open_settings', args: {} }]);

    bridge.pushView({ ...fixtureView('empty'), update_available: '1.3.0' });
    expect(text(root.querySelector('.update-mark'))).toBe('Update 1.3.0');
    expect(root.querySelector('button.settings')?.getAttribute('aria-label')).toBe('Settings, update 1.3.0 available');
  });

  it('tauri_bridge_subscribes_then_asks', async () => {
    const asked: string[] = [];
    mockIPC(
      (cmd, payload) => {
        asked.push(cmd);
        if (cmd === 'get_view') return fixtureView('empty');
        if (cmd === 'grab_text') {
          expect(payload).toEqual({ text: 'Item Class: Rings' });
          return fixtureView('grabbed-not-item');
        }
        return undefined;
      },
      { shouldMockEvents: true },
    );
    const views: View[] = [];
    const bridge = tauriBridge();
    const off = await bridge.start((v) => views.push(v));
    expect(asked).toContain('get_view');
    expect(views.map((v) => v.grab)).toEqual([null]);

    // A view the shell pushes on `ch:view` is delivered ...
    expect(VIEW_EVENT).toBe('ch:view');
    await emit(VIEW_EVENT, fixtureView('grabbed-item'));
    expect(views).toHaveLength(2);

    // ... and a command's own answer is not (the event carries the change).
    await bridge.command('grab_text', { text: 'Item Class: Rings' });
    expect(asked).toContain('grab_text');
    expect(views).toHaveLength(2);

    off();
    await emit(VIEW_EVENT, fixtureView('overlay-active'));
    expect(views).toHaveLength(2);
  });

  it('preview_reads_the_state_from_the_query', async () => {
    expect(previewViewName('')).toBe(DEFAULT_PREVIEW_VIEW);
    expect(previewViewName('?state=hotkey-taken')).toBe('hotkey-taken');
    expect(previewViewName('?state=sideways')).toBe(DEFAULT_PREVIEW_VIEW);
    expect(previewViewName('?state=sideways', 'overlay-active')).toBe('overlay-active');
    for (const name of FIXTURE_NAMES) expect(previewViewName(`?state=${name}`)).toBe(name);

    vi.spyOn(console, 'info').mockImplementation(() => {});
    const bridge = previewBridge('?state=grabbed-not-item');
    const r = await renderComponent(App, { bridge }, (root) => root.querySelector('.app[data-state]') !== null);
    mounted.push(r);
    expect(text(r.root.querySelector('.count'))).toBe('3 lines');
    // A paste in the preview is logged, not acted on: the view stays the fixture's.
    paste(document.body, 'anything');
    expect(bridge.calls).toEqual([{ name: 'grab_text', args: { text: 'anything' } }]);
    expect(text(r.root.querySelector('.count'))).toBe('3 lines');
  });
});
