import { flushSync } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import type { View } from './ipc';
import { type Rendered, fakeBridge, fixtureView, renderApp } from './test-support';

// The "Read the screen" setting: the opt-in screen reader's toggle in the
// settings frame. Off by default; the toggle sends `update_settings` with
// `screen_reading` alone, and play mode lists the value with no control.

let rendered: Rendered | null = null;

afterEach(() => {
  rendered?.destroy();
  rendered = null;
});

function settingsView(mode: 'play' | 'edit', screenReading?: boolean): View {
  const view = fixtureView('settings', mode);
  view.frame = 'settings';
  if (screenReading !== undefined) view.settings.screen_reading = screenReading;
  return view;
}

async function render(view: View) {
  const bridge = fakeBridge(view);
  rendered = await renderApp(bridge);
  return { root: rendered.root, bridge };
}

function rowByLabel(root: HTMLElement, label: string): HTMLElement {
  const row = [...root.querySelectorAll<HTMLElement>('.row')].find((r) => r.querySelector('.label, dt')?.textContent?.trim() === label);
  expect(row, label).toBeTruthy();
  return row!;
}

describe('the "Read the screen" setting', () => {
  it('is_off_in_the_fixtures', () => {
    expect(fixtureView('settings').settings.screen_reading).toBe(false);
  });

  it('toggle_sends_update_settings_with_screen_reading', async () => {
    for (const on of [false, true]) {
      const { root, bridge } = await render(settingsView('edit', on));
      const row = rowByLabel(root, 'Read the screen');
      const toggle = row.querySelector<HTMLInputElement>('input[type="checkbox"].toggle[role="switch"]')!;
      expect(toggle, String(on)).not.toBeNull();
      expect(toggle.checked).toBe(on);
      // It sits right after "Only worth-it quests", with its helper line under it.
      expect(row.previousElementSibling?.querySelector('.label')?.textContent?.trim()).toBe('Only worth-it quests');
      const help = row.nextElementSibling!;
      expect(help.classList.contains('help')).toBe(true);
      expect(help.textContent).toMatch(/boss bars/);
      expect(help.textContent).toMatch(/dialogue/);
      expect(help.textContent).toMatch(/saves nothing/i);

      toggle.click();
      flushSync();
      expect(bridge.calls).toEqual([{ name: 'update_settings', args: { patch: { screen_reading: !on } } }]);
      rendered?.destroy();
      rendered = null;
    }
  });

  it('play_mode_lists_the_value', async () => {
    for (const [on, text] of [
      [false, 'Off'],
      [true, 'On'],
    ] as const) {
      const { root, bridge } = await render(settingsView('play', on));
      const row = rowByLabel(root, 'Read the screen');
      expect(row.querySelector('dd')?.textContent).toBe(text);
      expect(row.querySelector('input')).toBeNull();
      expect(bridge.calls).toEqual([]);
      rendered?.destroy();
      rendered = null;
    }
  });
});
