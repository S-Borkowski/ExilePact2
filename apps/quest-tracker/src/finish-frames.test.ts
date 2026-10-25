import { flushSync, mount, unmount } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import CharacterPicker from './components/CharacterPicker.svelte';
import Settings from './components/Settings.svelte';
import pickerSource from './components/CharacterPicker.svelte?raw';
import settingsSource from './components/Settings.svelte?raw';
import type { Send } from './bridge';
import type { Mode, View } from './ipc';
import { fixtureView } from './test-support';

// Finish review F9, F10 and F13 (frames): the Settings frame's copy and
// spacing, the character picker's active bar, and the two frame titles as
// headings. Views are built here from a cloned fixture, so the tests read
// none of the fixture's names, counts or levels.

const LONG_PATH =
  'D:/Games/A Very Long Library Folder Name/steamapps/common/Path of Exile 2/logs/Client.txt';

const mounted: Array<() => void> = [];

afterEach(() => {
  while (mounted.length) mounted.pop()?.();
});

const send: Send = () => {};

/** A settings view in `mode`, with `log_path` set as given. */
function settingsView(mode: Mode, logPath: string | null): View {
  const view = fixtureView('settings', mode);
  view.frame = 'settings';
  view.settings.log_path = logPath;
  return view;
}

/** A picker view in `mode` with one invented active character. */
function pickerView(mode: Mode): View {
  const view = fixtureView('character-picker', mode);
  view.frame = 'character-picker';
  const one = { name: 'InventedTester', class: 'Monk', level: 7, own: true, pinned: false, active: true };
  view.characters = [one];
  view.character = one;
  return view;
}

function render(component: typeof Settings | typeof CharacterPicker, view: View): HTMLElement {
  const target = document.createElement('div');
  document.body.appendChild(target);
  const app = mount(component, { target, props: { view, send } });
  flushSync();
  mounted.push(() => {
    unmount(app);
    target.remove();
  });
  return target;
}

/** The rule body of `selector` in a component's `<style>`. */
function rule(source: string, selector: string): string {
  const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const m = new RegExp(`(?:^|\\n)\\s*${escaped}\\s*\\{([^}]*)\\}`).exec(source);
  if (!m) throw new Error(`no rule ${selector}`);
  return m[1];
}

describe('finish review: frames', () => {
  it('settings_click_through_help_text', () => {
    const root = render(Settings, settingsView('edit', null));
    const toggle = Array.from(root.querySelectorAll('label.row')).find((l) => l.textContent?.includes('Click-through'));
    expect(toggle).toBeTruthy();
    const help = toggle?.nextElementSibling;
    expect(help?.classList.contains('help')).toBe(true);
    expect(help?.textContent?.trim()).toBe('On: in play mode the panel passes clicks to the game.');
    expect(root.textContent).not.toContain('On returns to play mode');

    // The help under the stacked log-file field sits 4 px below it.
    const fieldHelp = root.querySelector('label.field + p.help');
    expect(fieldHelp?.textContent?.trim()).toBe('Empty: found automatically.');
    expect(rule(settingsSource, '.field + .help')).toMatch(/margin-top:\s*4px/);
  });

  it('settings_long_log_path_keeps_full_title', () => {
    const edit = render(Settings, settingsView('edit', LONG_PATH));
    const input = edit.querySelector<HTMLInputElement>('label.field input.text-input');
    expect(input?.value).toBe(LONG_PATH);
    expect(input?.getAttribute('title')).toBe(LONG_PATH);
    expect(input?.classList.contains('path-input')).toBe(true);
    expect(rule(settingsSource, '.path-input')).toMatch(/text-overflow:\s*ellipsis/);

    const play = render(Settings, settingsView('play', LONG_PATH));
    expect(play.querySelector('dd.path')?.getAttribute('title')).toBe(LONG_PATH);

    // Negative control: with no path there is no title to show.
    const auto = render(Settings, settingsView('edit', null));
    expect(auto.querySelector('label.field input.text-input')?.hasAttribute('title')).toBe(false);
  });

  it('settings_and_picker_titles_are_h2', () => {
    for (const mode of ['play', 'edit'] as const) {
      const settings = render(Settings, settingsView(mode, null));
      const sTitle = settings.querySelector('.frame-head h2.frame-title');
      expect(sTitle?.textContent?.trim()).toBe('Settings');

      const picker = render(CharacterPicker, pickerView(mode));
      const pTitle = picker.querySelector('.frame-head h2.frame-title');
      expect(pTitle?.textContent?.trim()).toBe('Characters');
      expect(picker.querySelector('span.frame-title')).toBeNull();
    }
    // Visually unchanged: the heading drops the browser's default margin.
    expect(rule(settingsSource, '.frame-title')).toMatch(/margin:\s*0;/);
    expect(rule(pickerSource, '.frame-title')).toMatch(/margin:\s*0;/);
    // F10: the active bar sits inside the 2 px edit border.
    expect(rule(pickerSource, '.character.active::before')).toMatch(/left:\s*3px/);
  });
});
