// The Settings window (`settings.html`): every app-settings view the Rust
// side writes under `fixtures/app-settings/` (and its `install/`) renders and
// passes the strict checker, each control sends its command, no state offers
// a release page, and nothing on the page reaches the network.

import { flushSync, mount, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { emit } from '@tauri-apps/api/event';
import SettingsWindow from './SettingsWindow.svelte';
import { APP_SETTINGS_EVENT, type SettingsBridge, type SettingsCommand, type SettingsCommandArgs, tauriSettingsBridge } from './bridge';
import { parseSettingsPreviewQuery, previewSettingsBridge } from './preview';
import { SECTIONS } from './sections';
import { type AppSettingsView, INSTALL_STATES, type InstallState, UPDATE_STATES, appSettingsProblems } from './view';

const RAW = import.meta.glob('../../fixtures/app-settings/*.json', { eager: true, import: 'default' }) as Record<string, unknown>;
const RAW_INSTALL = import.meta.glob('../../fixtures/app-settings/install/*.json', {
  eager: true,
  import: 'default',
}) as Record<string, unknown>;

const byName = (raw: Record<string, unknown>): Record<string, unknown> =>
  Object.fromEntries(Object.entries(raw).map(([path, json]) => [path.replace(/^.*\/([^/]+)\.json$/, '$1'), json]));

/** Each fixture file by name (`idle`, `available`, ...), as parsed JSON. */
const FIXTURES: Record<string, unknown> = byName(RAW);
/** Each `install/` fixture by install state (`ready`, `downloading`, ...). */
const INSTALL_FIXTURES: Record<string, unknown> = byName(RAW_INSTALL);

function fixture(name: string): AppSettingsView {
  const json = FIXTURES[name];
  if (json === undefined) throw new Error(`no fixture app-settings/${name}.json`);
  return JSON.parse(JSON.stringify(json)) as AppSettingsView;
}

function installFixture(state: InstallState): AppSettingsView {
  const json = INSTALL_FIXTURES[state];
  if (json === undefined) throw new Error(`no fixture app-settings/install/${state}.json`);
  return JSON.parse(JSON.stringify(json)) as AppSettingsView;
}

/** Every fixture, the update states' and the install states', with a name for messages. */
const everyFixture = (): Array<[string, AppSettingsView]> => [
  ...UPDATE_STATES.map((s): [string, AppSettingsView] => [s, fixture(s)]),
  ...INSTALL_STATES.map((s): [string, AppSettingsView] => [`install/${s}`, installFixture(s)]),
];

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

/** The install part's own status line (empty where the install has nothing to say). */
const installLine = (root: HTMLElement): HTMLElement | null => root.querySelector('.install [role="status"]');

/** No control, link or line on the page that offers a release page or a browser. */
function expectNoPage(root: HTMLElement, name: string): void {
  expect(root.querySelector('a'), name).toBeNull();
  for (const b of root.querySelectorAll('button')) {
    expect(b.textContent ?? '', name).not.toMatch(/release page|browser|website|download page/i);
  }
  expect(root.querySelector('[data-icon="ui-external-link"]'), name).toBeNull();
}

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

    // One fixture per install state, and nothing else; each is a whole view
    // and renders its install part.
    expect(Object.keys(INSTALL_FIXTURES).sort()).toEqual([...INSTALL_STATES].sort());
    for (const state of INSTALL_STATES) {
      const view = installFixture(state);
      expect(appSettingsProblems(view), `install/${state}.json`).toEqual([]);
      expect(view.install.state).toBe(state);

      const root = await render(fakeBridge(view));
      expect(root.querySelector('.install')?.getAttribute('data-install') ?? 'none', state).toBe(state);
      // The check's status line stays the check's; the install says its own in
      // a second polite line, there (empty) in every state so that a screen
      // reader already watches it when the install's first words arrive.
      expect(root.querySelector('[role="status"]')?.textContent?.trim()).toBe(view.update.text);
      const line = installLine(root);
      expect(line, state).not.toBeNull();
      expect(line?.getAttribute('aria-live'), state).toBe('polite');
      expect(line?.textContent?.trim(), state).toBe(state === 'none' || state === 'ready' ? '' : view.install.text);
      expect(root.querySelector('footer')?.textContent?.trim()).toBe('Created by ST4H');
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

  it('toggle_sends_its_own_state_when_clicked_twice_before_a_view', async () => {
    // The shell has not answered the first click when the second comes: the
    // second sends what the switch now shows (off), not the negation of the
    // last view (which would send on a second time and lose the click).
    const off = fixture('idle');
    const bridge = fakeBridge(off);
    const root = await render(bridge);
    toggle(root).click();
    flushSync();
    expect(toggle(root).checked).toBe(true);
    toggle(root).click();
    flushSync();
    expect(bridge.calls).toEqual([
      { name: 'update_settings', args: { patch: { check_updates: true } } },
      { name: 'update_settings', args: { patch: { check_updates: false } } },
    ]);

    // The views then arrive in order, and the switch ends on the last one.
    bridge.push({ ...off, check_updates: true });
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

  it('install_button_only_with_a_newer_version', async () => {
    // The install button is the one primary button, labelled by the app
    // ("Install 1.3.0 and restart"), and only in `ready`: a newer, signed
    // release. No state offers a release page instead.
    for (const [name, view] of everyFixture()) {
      const root = await render(fakeBridge(view));
      const primary = [...root.querySelectorAll('button.primary')];
      if (view.install.state === 'ready') {
        expect(view.update.newer, name).not.toBeNull();
        expect(primary.map((b) => b.textContent?.trim()), name).toEqual([view.install.text]);
        expect(view.install.text, name).toMatch(new RegExp(`^Install ${view.update.newer!.replace(/\./g, '\\.')}\\b`));
      } else {
        expect(primary, name).toEqual([]);
      }
      expectNoPage(root, name);
    }
    // Both sides are exercised: `available` and the `failed` check that keeps a
    // newer version show it; with none known there is none.
    expect(UPDATE_STATES.filter((s) => fixture(s).install.state === 'ready').sort()).toEqual(['available', 'failed']);
    expect(UPDATE_STATES.filter((s) => fixture(s).update.newer === null).every((s) => fixture(s).install.state === 'none')).toBe(true);

    // A newer release that is not signed: its one line, no button, no Try again.
    const unsigned = installFixture('unsigned');
    const unsignedRoot = await render(fakeBridge(unsigned));
    expect(installLine(unsignedRoot)?.textContent?.trim()).toBe(
      `${unsigned.update.newer} is available but cannot be installed: the release is not signed.`,
    );
    expect(button(unsignedRoot, 'Try again')).toBeUndefined();
    expect(unsignedRoot.querySelector('button.primary')).toBeNull();
    expect(button(unsignedRoot, 'Check now')?.disabled).toBe(false);

    // Once no newer version is known, the button goes.
    const bridge = fakeBridge(fixture('available'));
    const root = await render(bridge);
    const label = fixture('available').install.text;
    expect(button(root, label)).toBeDefined();
    bridge.push(fixture('up-to-date'));
    expect(button(root, label)).toBeUndefined();
  });

  it('install_button_sends_install_update', async () => {
    const bridge = fakeBridge(installFixture('ready'));
    const root = await render(bridge);
    const install = button(root, installFixture('ready').install.text)!;
    expect(install.disabled).toBe(false);
    install.click();
    flushSync();
    expect(bridge.calls).toEqual([{ name: 'install_update', args: {} }]);

    // Negative control: while a check runs the app refuses an install, so the
    // button is disabled and a click sends nothing.
    const ready = installFixture('ready');
    const checking = { ...ready, update: { ...fixture('checking').update, newer: ready.update.newer } };
    expect(appSettingsProblems(checking)).toEqual([]);
    const busy = fakeBridge(checking);
    const busyRoot = await render(busy);
    const busyInstall = button(busyRoot, ready.install.text)!;
    expect(busyInstall.disabled).toBe(true);
    busyInstall.click();
    flushSync();
    expect(busy.calls).toEqual([]);

    // The next view takes over: once the download starts, the button is gone.
    bridge.push(installFixture('downloading'));
    expect(button(root, ready.install.text)).toBeUndefined();
  });

  it('download_progress_is_shown_with_cancel', async () => {
    const view = installFixture('downloading');
    const total = view.install.progress!.total!;
    expect(total).toBeGreaterThan(0);
    const bridge = fakeBridge(view);
    const root = await render(bridge);

    // A determinate bar: the release's size is known.
    const bar = root.querySelector('[role="progressbar"]')!;
    expect(bar).not.toBeNull();
    expect(bar.getAttribute('aria-valuemin')).toBe('0');
    expect(bar.getAttribute('aria-valuemax')).toBe(String(total));
    expect(bar.getAttribute('aria-valuenow')).toBe(String(view.install.progress!.bytes));
    expect(bar.getAttribute('aria-labelledby')).toBe(installLine(root)?.id);
    // The amount in words too, not the bar alone.
    expect(root.querySelector('.install .amount')?.textContent?.trim()).toBe('0 KB of 4 KB');
    expect(bar.getAttribute('aria-valuetext')).toBe('0 KB of 4 KB');

    // "Check now" waits for the install; *Cancel* stops it.
    expect(button(root, 'Check now')?.disabled).toBe(true);
    button(root, 'Cancel')!.click();
    flushSync();
    expect(bridge.calls).toEqual([{ name: 'cancel_update', args: {} }]);

    // Progress moves with the views.
    const half = { ...view, install: { ...view.install, progress: { bytes: 2048, total } } };
    bridge.push(half);
    expect(bar.getAttribute('aria-valuenow')).toBe('2048');
    expect(root.querySelector('.install .amount')?.textContent?.trim()).toBe('2 KB of 4 KB');
    const big = { ...view, install: { ...view.install, progress: { bytes: 5_452_595, total: 12_582_912 } } };
    bridge.push(big);
    expect(root.querySelector('.install .amount')?.textContent?.trim()).toBe('5.2 MB of 12.0 MB');

    // Without a total the bar is indeterminate: no value, the amount alone.
    const unknown = { ...view, install: { ...view.install, progress: { bytes: 2048, total: null } } };
    expect(appSettingsProblems(unknown)).toEqual([]);
    bridge.push(unknown);
    const open = root.querySelector('[role="progressbar"]')!;
    expect(open.hasAttribute('aria-valuenow')).toBe(false);
    expect(open.hasAttribute('aria-valuemax')).toBe(false);
    expect(root.querySelector('.install .amount')?.textContent?.trim()).toBe('2 KB');

    // A cancel goes back to the install button; no bar, no Cancel.
    bridge.push(installFixture('ready'));
    expect(root.querySelector('[role="progressbar"]')).toBeNull();
    expect(button(root, 'Cancel')).toBeUndefined();
    expect(button(root, installFixture('ready').install.text)).toBeDefined();

    // Verifying and restarting: the line alone, nothing to press but nothing to check either.
    for (const state of ['verifying', 'restarting'] as const) {
      const r = await render(fakeBridge(installFixture(state)));
      expect(installLine(r)?.textContent?.trim(), state).toBe(installFixture(state).install.text);
      expect(r.querySelector('[role="progressbar"]'), state).toBeNull();
      expect([...r.querySelectorAll('button')].map((b) => b.textContent?.trim()), state).toEqual(['Check now']);
      expect(button(r, 'Check now')?.disabled, state).toBe(true);
    }
  });

  it('failure_shows_the_reason_and_try_again_without_a_page', async () => {
    const view = installFixture('failed');
    const bridge = fakeBridge(view);
    const root = await render(bridge);

    // The reason, in one polite line, said in words (the icon only repeats it).
    const line = installLine(root)!;
    expect(line.textContent?.trim()).toBe(view.install.text);
    expect(view.install.text).toMatch(/^Could not install .+: .+\.$/);
    expect(line.getAttribute('aria-live')).toBe('polite');
    expect(line.querySelector('[data-icon="ui-warning"]')).not.toBeNull();

    // *Try again* re-runs the install; "Check now" stays.
    expect(root.querySelector('button.primary')).toBeNull();
    expect(button(root, 'Check now')?.disabled).toBe(false);
    button(root, 'Try again')!.click();
    flushSync();
    expect(bridge.calls).toEqual([{ name: 'install_update', args: {} }]);
    button(root, 'Check now')!.click();
    flushSync();
    expect(bridge.calls.map((c) => c.name)).toEqual(['install_update', 'check_for_updates']);

    // No page, no browser, in this state or any other.
    for (const [name, v] of everyFixture()) expectNoPage(await render(fakeBridge(v)), name);

    // Positive control: the check does see a release-page button or a link.
    const probe = document.createElement('div');
    probe.innerHTML = '<button>Open the release page</button>';
    expect(() => expectNoPage(probe, 'probe')).toThrow();
    probe.innerHTML = '<a href="https://github.com">release</a>';
    expect(() => expectNoPage(probe, 'probe')).toThrow();

    // Negative control: *Try again* belongs to a failed install only.
    for (const [name, v] of everyFixture()) {
      if (v.install.state === 'failed') continue;
      expect(button(await render(fakeBridge(v)), 'Try again'), name).toBeUndefined();
    }
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
    const clickEverything = async (root: HTMLElement) => {
      toggle(root).click();
      for (const b of root.querySelectorAll('button')) b.click();
      flushSync();
      await new Promise((resolve) => setTimeout(resolve, 0));
    };
    for (const state of UPDATE_STATES) {
      for (const bridge of [fakeBridge(fixture(state)), previewSettingsBridge(`?state=${state}`)]) {
        vi.spyOn(console, 'info').mockImplementation(() => {});
        const root = await render(bridge);
        expect(root.querySelector('.window')?.getAttribute('data-state')).toBe(state);
        await clickEverything(root);
      }
    }
    // The install states too, the install button, Cancel and Try again among them.
    for (const state of INSTALL_STATES) {
      for (const bridge of [fakeBridge(installFixture(state)), previewSettingsBridge(`?install=${state}`)]) {
        vi.spyOn(console, 'info').mockImplementation(() => {});
        const root = await render(bridge);
        expect(root.querySelector('.install')?.getAttribute('data-install') ?? 'none').toBe(state);
        await clickEverything(root);
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

    // The install part: a browser link smuggled in, a bad state, a missing progress field.
    const page = installFixture('failed') as unknown as { install: Record<string, unknown> };
    page.install.browser_url = 'https://github.com';
    expect(appSettingsProblems(page)).toEqual(['app_settings.install.browser_url: not in the TypeScript types']);
    const badInstall = installFixture('ready') as unknown as { install: { state: string } };
    badInstall.install.state = 'open-page';
    expect(appSettingsProblems(badInstall)).toEqual([
      `app_settings.install.state: "open-page" is not one of ${INSTALL_STATES.join('|')}`,
    ]);
    const noTotal = installFixture('downloading') as unknown as { install: { progress: Record<string, unknown> } };
    delete noTotal.install.progress.total;
    expect(appSettingsProblems(noTotal)).toEqual(['app_settings.install.progress.total: missing']);
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
    expect(parseSettingsPreviewQuery('')).toEqual({ state: 'idle', install: null, scale: null });
    expect(parseSettingsPreviewQuery('?state=available&scale=2')).toEqual({ state: 'available', install: null, scale: '2' });
    expect(parseSettingsPreviewQuery('?state=sideways&scale=3')).toEqual({ state: 'idle', install: null, scale: null });
    expect(parseSettingsPreviewQuery('?install=downloading')).toEqual({ state: 'idle', install: 'downloading', scale: null });
    expect(parseSettingsPreviewQuery('?install=open-page')).toEqual({ state: 'idle', install: null, scale: null });

    const root = await render(previewSettingsBridge('?state=failed&scale=2'));
    expect(root.querySelector('.window')?.getAttribute('data-state')).toBe('failed');
    expect(document.documentElement.dataset.previewScale).toBe('2');

    // `install` picks an `install/` fixture, whatever `state` says.
    const installing = await render(previewSettingsBridge('?state=failed&install=failed'));
    expect(installing.querySelector('.install')?.getAttribute('data-install')).toBe('failed');
    expect(installing.querySelector('.window')?.getAttribute('data-state')).toBe('available');
  });
});
