// The Settings window (`settings.html`): every app-settings view the Rust
// side writes under `fixtures/app-settings/` renders and passes the strict
// checker, each control sends its command, and nothing on the page reaches
// the network.

import { flushSync, mount, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { emit } from '@tauri-apps/api/event';
import SettingsWindow from './SettingsWindow.svelte';
import { APP_SETTINGS_EVENT, type SettingsBridge, type SettingsCommand, type SettingsCommandArgs, tauriSettingsBridge } from './bridge';
import { parseSettingsPreviewQuery, previewSettingsBridge } from './preview';
import { SECTIONS } from './sections';
import { type AppSettingsView, UPDATE_STATES, appSettingsProblems } from './view';

const RAW = import.meta.glob('../../fixtures/app-settings/*.json', { eager: true, import: 'default' }) as Record<string, unknown>;

/** Each fixture file by name (`idle`, `available`, ...), as parsed JSON. */
const FIXTURES: Record<string, unknown> = Object.fromEntries(
  Object.entries(RAW).map(([path, json]) => [path.replace(/^.*\/([^/]+)\.json$/, '$1'), json]),
);

function fixture(name: string): AppSettingsView {
  const json = FIXTURES[name];
  if (json === undefined) throw new Error(`no fixture app-settings/${name}.json`);
  return JSON.parse(JSON.stringify(json)) as AppSettingsView;
}

interface FakeSettingsBridge extends SettingsBridge {
  calls: Array<{ name: SettingsCommand; args: unknown }>;
  push(view: AppSettingsView): void;
}

/** A bridge that delivers views as the shell would and records every command. */
function fakeBridge(initial: AppSettingsView): FakeSettingsBridge {
  let onView: ((view: AppSettingsView) => void) | null = null;
  const calls: Array<{ name: SettingsCommand; args: unknown }> = [];
  return {
    kind: 'tauri',
    calls,
    async start(v) {
      onView = v;
      v(initial);
      return () => {
        onView = null;
      };
    },
    async command<C extends SettingsCommand>(name: C, args: SettingsCommandArgs[C]) {
      calls.push({ name, args });
    },
    push(view) {
      onView?.(view);
      flushSync();
    },
  };
}

let destroyers: Array<() => void> = [];

/** Mounts the window on `bridge` and lets it take its first view. */
async function render(bridge: SettingsBridge): Promise<HTMLElement> {
  const target = document.createElement('div');
  document.body.appendChild(target);
  const app = mount(SettingsWindow, { target, props: { bridge } });
  destroyers.push(() => {
    unmount(app);
    target.remove();
  });
  flushSync();
  for (let i = 0; i < 200 && target.querySelector('.window')?.getAttribute('data-state') === 'none'; i++) {
    await new Promise((resolve) => setTimeout(resolve, 5));
    flushSync();
  }
  return target;
}

const button = (root: HTMLElement, text: string): HTMLButtonElement | undefined =>
  [...root.querySelectorAll('button')].find((b) => b.textContent?.trim() === text);

const toggle = (root: HTMLElement): HTMLInputElement => root.querySelector('input[role="switch"]') as HTMLInputElement;

afterEach(() => {
  for (const destroy of destroyers) destroy();
  destroyers = [];
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  clearMocks();
  delete document.documentElement.dataset.previewScale;
});

describe('the Settings window', () => {
  it('renders_every_app_settings_fixture', async () => {
    // One fixture per update state, and nothing else.
    expect(Object.keys(FIXTURES).sort()).toEqual([...UPDATE_STATES].sort());

    for (const state of UPDATE_STATES) {
      const view = fixture(state);
      expect(appSettingsProblems(view), `${state}.json`).toEqual([]);
      expect(view.update.state).toBe(state);

      const root = await render(fakeBridge(view));
      const win = root.querySelector('.window')!;
      expect(win.getAttribute('data-state'), state).toBe(state);
      expect(root.querySelector('h1')?.textContent).toBe('Settings');

      // One region per section, in order, named by its heading.
      const regions = [...root.querySelectorAll('section')];
      expect(regions.map((r) => r.querySelector('h2')?.textContent)).toEqual(SECTIONS.map((s) => s.title));
      for (const region of regions) {
        const heading = region.querySelector('h2')!;
        expect(region.getAttribute('aria-labelledby')).toBe(heading.id);
      }

      // The switch shows the setting, named by its label and described by its help.
      const sw = toggle(root);
      expect(sw.checked, state).toBe(view.check_updates);
      expect(sw.closest('label')?.textContent).toContain('Automatically check for updates');
      const help = document.getElementById(sw.getAttribute('aria-describedby')!);
      expect(help?.textContent).toMatch(/downloads nothing/i);

      // The status line is the app's text, in a polite live region.
      const status = root.querySelector('[role="status"]')!;
      expect(status.textContent?.trim()).toBe(view.update.text);
      expect(status.getAttribute('aria-live')).toBe('polite');

      // "Check now" is there in every state, disabled only while checking.
      expect(button(root, 'Check now')?.disabled, state).toBe(state === 'checking');

      // The version line and the credit.
      expect(root.querySelector('.version')?.textContent).toBe(`Version ${view.version}`);
      expect(root.querySelector('footer')?.textContent?.trim()).toBe('Created by ST4H');
      expect(root.querySelector('footer a')).toBeNull();
      // The credit is the window's footer, outside its main region.
      expect(root.querySelector('main footer')).toBeNull();
    }

    // A new view replaces the old one in place.
    const bridge = fakeBridge(fixture('idle'));
    const root = await render(bridge);
    bridge.push(fixture('checking'));
    expect(root.querySelector('[role="status"]')?.textContent?.trim()).toBe(fixture('checking').update.text);
    expect(button(root, 'Check now')?.disabled).toBe(true);
  });

  it('toggle_sends_update_settings_with_check_updates', async () => {
    // Off -> sends on.
    const off = fixture('idle');
    expect(off.check_updates).toBe(false);
    const bridge = fakeBridge(off);
    const root = await render(bridge);
    toggle(root).click();
    flushSync();
    expect(bridge.calls).toEqual([{ name: 'update_settings', args: { patch: { check_updates: true } } }]);

    // On -> sends off.
    const on = fixture('available');
    expect(on.check_updates).toBe(true);
    const bridge2 = fakeBridge(on);
    const root2 = await render(bridge2);
    toggle(root2).click();
    flushSync();
    expect(bridge2.calls).toEqual([{ name: 'update_settings', args: { patch: { check_updates: false } } }]);

    // The switch follows the next view, not its own click.
    bridge.push({ ...off, check_updates: true });
    expect(toggle(root).checked).toBe(true);
    bridge.push(off);
    expect(toggle(root).checked).toBe(false);
  });

  it('check_now_sends_check_for_updates', async () => {
    const bridge = fakeBridge(fixture('up-to-date'));
    const root = await render(bridge);
    button(root, 'Check now')!.click();
    flushSync();
    expect(bridge.calls).toEqual([{ name: 'check_for_updates', args: {} }]);

    // Negative control: while a check runs, the button is disabled and a click sends nothing.
    const busy = fakeBridge(fixture('checking'));
    const busyRoot = await render(busy);
    button(busyRoot, 'Check now')!.click();
    flushSync();
    expect(busy.calls).toEqual([]);
  });

  it('release_page_button_only_with_a_newer_version', async () => {
    for (const state of UPDATE_STATES) {
      const view = fixture(state);
      const bridge = fakeBridge(view);
      const root = await render(bridge);
      const open = button(root, 'Open the release page');
      if (view.update.newer === null) {
        expect(open, state).toBeUndefined();
      } else {
        expect(open, state).toBeDefined();
        open!.click();
        flushSync();
        expect(bridge.calls).toEqual([{ name: 'open_release_page', args: {} }]);
      }
    }
    // Both sides are exercised: `available` and the `failed` that keeps a newer version show it.
    expect(UPDATE_STATES.filter((s) => fixture(s).update.newer !== null).sort()).toEqual(['available', 'failed']);

    // Once no newer version is known, the button goes.
    const bridge = fakeBridge(fixture('available'));
    const root = await render(bridge);
    expect(button(root, 'Open the release page')).toBeDefined();
    bridge.push(fixture('up-to-date'));
    expect(button(root, 'Open the release page')).toBeUndefined();
  });

  it('makes_no_network_request', async () => {
    const requests: string[] = [];
    vi.stubGlobal('fetch', (...args: unknown[]) => {
      requests.push(`fetch ${String(args[0])}`);
      return Promise.reject(new Error('no network in this test'));
    });
    vi.stubGlobal(
      'XMLHttpRequest',
      class {
        open(method: string, url: string) {
          requests.push(`xhr ${method} ${url}`);
        }
        send() {
          requests.push('xhr send');
        }
      },
    );
    vi.stubGlobal(
      'WebSocket',
      class {
        constructor(url: string) {
          requests.push(`ws ${url}`);
        }
      },
    );

    // Positive control: the recorders do see a request made through each door.
    void fetch('probe').catch(() => {});
    new XMLHttpRequest().open('GET', 'probe');
    new WebSocket('probe');
    expect(requests).toEqual(['fetch probe', 'xhr GET probe', 'ws probe']);
    requests.length = 0;

    // Every fixture rendered, through the app's bridge shape and the preview's,
    // and every control on it clicked.
    for (const state of UPDATE_STATES) {
      for (const bridge of [fakeBridge(fixture(state)), previewSettingsBridge(`?state=${state}`)]) {
        vi.spyOn(console, 'info').mockImplementation(() => {});
        const root = await render(bridge);
        expect(root.querySelector('.window')?.getAttribute('data-state')).toBe(state);
        toggle(root).click();
        for (const b of root.querySelectorAll('button')) b.click();
        flushSync();
        await new Promise((resolve) => setTimeout(resolve, 0));
      }
    }
    expect(requests).toEqual([]);
  });

  it('app_settings_problems_catch_drift', () => {
    expect(appSettingsProblems(fixture('available'))).toEqual([]);

    // An extra key, at the top and inside `update`.
    const extra = fixture('idle') as unknown as Record<string, unknown>;
    extra.channel = 'beta';
    expect(appSettingsProblems(extra)).toEqual(['app_settings.channel: not in the TypeScript types']);
    const extraInner = fixture('idle') as unknown as { update: Record<string, unknown> };
    extraInner.update.url = 'x';
    expect(appSettingsProblems(extraInner)).toEqual(['app_settings.update.url: not in the TypeScript types']);

    // A missing key.
    const missing = fixture('idle') as unknown as Record<string, unknown>;
    delete missing.check_updates;
    expect(appSettingsProblems(missing)).toEqual(['app_settings.check_updates: missing']);
    const missingInner = fixture('available') as unknown as { update: Record<string, unknown> };
    delete missingInner.update.newer;
    expect(appSettingsProblems(missingInner)).toEqual(['app_settings.update.newer: missing']);

    // A bad state, and a wrong type.
    const badState = fixture('idle') as unknown as { update: { state: string } };
    badState.update.state = 'up_to_date';
    expect(appSettingsProblems(badState)).toEqual([
      `app_settings.update.state: "up_to_date" is not one of ${UPDATE_STATES.join('|')}`,
    ]);
    const badType = fixture('idle') as unknown as Record<string, unknown>;
    badType.check_updates = 'false';
    expect(appSettingsProblems(badType)).toEqual(['app_settings.check_updates: expected a boolean']);
    expect(appSettingsProblems(null)).toEqual(['app_settings: expected an object']);
  });

  it('tauri_bridge_subscribes_then_asks', async () => {
    const asked: string[] = [];
    mockIPC(
      (cmd, payload) => {
        asked.push(cmd);
        if (cmd === 'get_app_settings') return fixture('idle');
        if (cmd === 'check_for_updates') {
          expect(payload).toEqual({});
          return fixture('checking');
        }
        return undefined;
      },
      { shouldMockEvents: true },
    );
    const views: AppSettingsView[] = [];
    const bridge = tauriSettingsBridge();
    const off = await bridge.start((v) => views.push(v));
    expect(asked).toContain('get_app_settings');
    expect(views.map((v) => v.update.state)).toEqual(['idle']);

    // A view the shell sends on `qt:app-settings` is rendered ...
    await emit(APP_SETTINGS_EVENT, fixture('up-to-date'));
    expect(views.map((v) => v.update.state)).toEqual(['idle', 'up-to-date']);

    // ... and a command's own answer is not (the events carry every change).
    await bridge.command('check_for_updates', {});
    expect(asked).toContain('check_for_updates');
    expect(views).toHaveLength(2);

    off();
    await emit(APP_SETTINGS_EVENT, fixture('available'));
    expect(views).toHaveLength(2);
  });

  it('preview_reads_the_state_from_the_query', async () => {
    expect(parseSettingsPreviewQuery('')).toEqual({ state: 'idle', scale: null });
    expect(parseSettingsPreviewQuery('?state=available&scale=2')).toEqual({ state: 'available', scale: '2' });
    expect(parseSettingsPreviewQuery('?state=sideways&scale=3')).toEqual({ state: 'idle', scale: null });

    const root = await render(previewSettingsBridge('?state=failed&scale=2'));
    expect(root.querySelector('.window')?.getAttribute('data-state')).toBe('failed');
    expect(document.documentElement.dataset.previewScale).toBe('2');
  });
});
