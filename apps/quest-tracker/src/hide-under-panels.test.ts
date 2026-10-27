import { flushSync } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import type { View } from './ipc';
import { type Rendered, fakeBridge, fixtureView, renderApp } from './test-support';

// The "Hide under game panels" setting: the opt-in panel watcher's toggle in
// the settings frame. Off by default; the toggle sends `update_settings` with
// `hide_under_panels` alone, and play mode lists the value with no control.

let rendered: Rendered | null = null;

afterEach(() => {
  rendered?.destroy();
  rendered = null;
});

function settingsView(mode: 'play' | 'edit', hideUnderPanels?: boolean): View {
  const view = fixtureView('settings', mode);
  view.frame = 'settings';
  if (hideUnderPanels !== undefined) view.settings.hide_under_panels = hideUnderPanels;
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

describe('the "Hide under game panels" setting', () => {
  it('is_off_in_the_fixtures', () => {
    expect(fixtureView('settings').settings.hide_under_panels).toBe(false);
  });

  it('toggle_sends_update_settings_with_hide_under_panels', async () => {
    for (const on of [false, true]) {
      const { root, bridge } = await render(settingsView('edit', on));
      const row = rowByLabel(root, 'Hide under game panels');
      const toggle = row.querySelector<HTMLInputElement>('input[type="checkbox"].toggle[role="switch"]')!;
      expect(toggle, String(on)).not.toBeNull();
      expect(toggle.checked).toBe(on);
      // It sits right after "Read the screen" and its helper line, with its own helper line under it.
      const before = row.previousElementSibling!;
      expect(before.classList.contains('help')).toBe(true);
      expect(before.previousElementSibling?.querySelector('.label')?.textContent?.trim()).toBe('Read the screen');
      const help = row.nextElementSibling!;
      expect(help.classList.contains('help')).toBe(true);
      expect(help.textContent).toMatch(/game panel is open/);
      expect(help.textContent).toMatch(/a few pixels/);
      expect(help.textContent).toMatch(/saves nothing/i);

      toggle.click();
      flushSync();
      expect(bridge.calls).toEqual([{ name: 'update_settings', args: { patch: { hide_under_panels: !on } } }]);
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
      const row = rowByLabel(root, 'Hide under game panels');
      expect(row.querySelector('dd')?.textContent).toBe(text);
      expect(row.querySelector('input')).toBeNull();
      // It follows "Read the screen" in the value list.
      expect(row.previousElementSibling?.querySelector('dt')?.textContent?.trim()).toBe('Read the screen');
      expect(bridge.calls).toEqual([]);
      rendered?.destroy();
      rendered = null;
    }
  });
});
