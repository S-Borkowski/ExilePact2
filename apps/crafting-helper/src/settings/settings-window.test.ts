// The Settings window (`settings.html`): every Settings view the Rust side
// writes under `fixtures/app-settings/` renders and passes the strict
// checker, each control sends its command (the hotkey form `set_hotkey`, the
// blocked currencies' picker, which offers only the catalogue's names, the
// whole list), an install's progress and its failure are
// shown in words with no release page anywhere, and nothing on the page
// reaches the network. The Updates section and its tests are the waystone
// roller's.

import { flushSync, mount, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { emit } from '@tauri-apps/api/event';
import SettingsWindow from './SettingsWindow.svelte';
import {
  APP_SETTINGS_EVENT,
  type SettingsCommand,
  type SettingsCommandArgs,
  type SettingsWindowBridge,
  tauriSettingsBridge,
} from './bridge';
import { PREVIEW_STATES, parseSettingsPreviewQuery, previewSettingsBridge } from './preview';
import { SECTIONS } from './sections';
import { CATALOGUE, choose, click, setValue } from '../test-support';
import type { CatalogueGroup } from '../ipc';
import { type SettingsView, INSTALL_STATES, UPDATE_STATES, settingsProblems } from './view';

const RAW = import.meta.glob('../../fixtures/app-settings/*.json', { eager: true, import: 'default' }) as Record<string, unknown>;

/** Each fixture file by name (`idle`, `install-failed`, ...), as parsed JSON. */
const FIXTURES: Record<string, unknown> = Object.fromEntries(
  Object.entries(RAW).map(([path, json]) => [path.replace(/^.*\/([^/]+)\.json$/, '$1'), json]),
);

function fixture(name: string): SettingsView {
  const json = FIXTURES[name];
  if (json === undefined) throw new Error(`no fixture app-settings/${name}.json`);
  return JSON.parse(JSON.stringify(json)) as SettingsView;
}

interface FakeSettingsBridge extends SettingsWindowBridge {
  calls: Array<{ name: SettingsCommand; args: unknown }>;
  /** How many times the window asked for the catalogue. */
  catalogueCalls: number;
  push(view: SettingsView): void;
}

/**
 * A bridge that delivers views as the shell would, records every command
 * and answers the catalogue with `fixtures/catalogue.json`; `null` delivers
 * no view.
 */
function fakeBridge(initial: SettingsView | null): FakeSettingsBridge {
  let onView: ((view: SettingsView) => void) | null = null;
  const calls: Array<{ name: SettingsCommand; args: unknown }> = [];
  return {
    kind: 'tauri',
    calls,
    catalogueCalls: 0,
    async catalogue() {
      this.catalogueCalls += 1;
      return JSON.parse(JSON.stringify(CATALOGUE)) as CatalogueGroup[];
    },
    async start(v) {
      onView = v;
      if (initial) v(initial);
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

/** Mounts the window on `bridge` and lets it take its first view (if the bridge sends one) and the catalogue. */
async function render(bridge: SettingsWindowBridge, wait = true): Promise<HTMLElement> {
  const target = document.createElement('div');
  document.body.appendChild(target);
  const app = mount(SettingsWindow, { target, props: { bridge } });
  destroyers.push(() => {
    unmount(app);
    target.remove();
  });
  flushSync();
  const waiting = () =>
    target.querySelector('.window')?.getAttribute('data-state') === 'none' ||
    target.querySelector('section[aria-labelledby="section-blocked"] input[data-id]') === null;
  for (let i = 0; wait && i < 200 && waiting(); i++) {
    await new Promise((resolve) => setTimeout(resolve, 5));
    flushSync();
  }
  return target;
}

const text = (el: Element | null | undefined) => (el?.textContent ?? '').replace(/\s+/g, ' ').trim();

const button = (root: HTMLElement, label: string): HTMLButtonElement | undefined =>
  [...root.querySelectorAll('button')].find((b) => text(b) === label);

/** The Updates section's switch (the Overlay section has its own). */
const toggle = (root: HTMLElement): HTMLInputElement =>
  root.querySelector('section[aria-labelledby="section-updates"] input[role="switch"]') as HTMLInputElement;
/** The Overlay section's region. */
const overlaySection = (root: HTMLElement): HTMLElement => root.querySelector('section[aria-labelledby="section-overlay"]') as HTMLElement;

/** The install states that write their own status line (all but `none` and `ready`). */
const INSTALL_LINE_STATES = ['unsigned', 'downloading', 'verifying', 'restarting', 'failed'];
/** The install states in which an install runs: no check and no second install then. */
const INSTALL_BUSY_STATES = ['downloading', 'verifying', 'restarting'];

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
    // One fixture per check state and per install state, and nothing else;
    // the preview offers exactly these.
    expect(Object.keys(FIXTURES).sort()).toEqual([...PREVIEW_STATES].sort());
    const seenUpdate = new Set<string>();
    const seenInstall = new Set<string>();

    for (const name of PREVIEW_STATES) {
      const view = fixture(name);
      expect(settingsProblems(view), `${name}.json`).toEqual([]);
      seenUpdate.add(view.update.state);
      seenInstall.add(view.install.state);

      const root = await render(fakeBridge(view));
      const win = root.querySelector('.window')!;
      expect(win.getAttribute('data-state'), name).toBe(view.update.state);
      expect(win.getAttribute('data-install'), name).toBe(view.install.state);
      expect(text(root.querySelector('h1'))).toBe('Settings');

      // One region per section, in order, named by its heading.
      const regions = [...root.querySelectorAll('section')];
      expect(regions.map((r) => text(r.querySelector('h2')))).toEqual(SECTIONS.map((s) => s.title));
      for (const region of regions) {
        expect(region.getAttribute('aria-labelledby')).toBe(region.querySelector('h2')!.id);
      }

      // The switch shows the setting, named by its label and described by its help.
      const sw = toggle(root);
      expect(sw.checked, name).toBe(view.check_updates);
      expect(sw.closest('label')?.textContent).toContain('Automatically check for updates');
      const help = document.getElementById(sw.getAttribute('aria-describedby')!);
      expect(help?.textContent).toMatch(/downloads nothing/i);

      // The Grab section shows the hotkey, and the picker checks the saved ids.
      expect(text(root.querySelector('kbd.hotkey')), name).toBe(view.hotkey);
      const checked = [...root.querySelectorAll<HTMLInputElement>('section[aria-labelledby="section-blocked"] input[data-id]')]
        .filter((box) => box.checked)
        .map((box) => box.dataset.id);
      expect(checked.sort(), name).toEqual([...view.blocked_currencies].sort());

      // The update status is the app's text, in one polite live region: the
      // check's line, then the install's own line when it has one.
      const status = root.querySelector('section[aria-labelledby="section-updates"] [role="status"]')!;
      expect(status.getAttribute('aria-live')).toBe('polite');
      // A failed check starts with "!", not told by its colour alone.
      const mark = view.update.state === 'failed' ? '!' : '';
      expect(text(status.querySelector('.check-line')), name).toBe(`${mark}${view.update.text}`);
      const installLine = status.querySelector('.install-line');
      if (INSTALL_LINE_STATES.includes(view.install.state)) {
        expect(text(installLine), name).toContain(view.install.text);
      } else {
        expect(installLine, name).toBeNull();
      }

      // "Check now" is there in every state, disabled while a check or an install runs.
      const busy = view.update.state === 'checking' || INSTALL_BUSY_STATES.includes(view.install.state);
      expect(button(root, 'Check now')?.disabled, name).toBe(busy);

      // The version line.
      expect(text(root.querySelector('.version'))).toBe(`Version ${view.version}`);

      // The Overlay section shows the saved position and the screen watch.
      const overlay = overlaySection(root);
      expect(overlay.querySelector<HTMLSelectElement>('select')!.value, name).toBe(view.overlay_position);
      expect(overlay.querySelector<HTMLInputElement>('input[role="switch"]')!.checked, name).toBe(view.screen_watch);
    }
    // The fixtures cover every state the types declare.
    expect([...seenUpdate].sort()).toEqual([...UPDATE_STATES].sort());
    expect([...seenInstall].sort()).toEqual([...INSTALL_STATES].sort());

    // A new view replaces the old one in place.
    const bridge = fakeBridge(fixture('idle'));
    const root = await render(bridge);
    bridge.push(fixture('checking'));
    expect(text(root.querySelector('.check-line'))).toBe(fixture('checking').update.text);
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

    // Two clicks before a view: the second sends what the switch now shows.
    const bridge3 = fakeBridge(off);
    const root3 = await render(bridge3);
    toggle(root3).click();
    flushSync();
    toggle(root3).click();
    flushSync();
    expect(bridge3.calls).toEqual([
      { name: 'update_settings', args: { patch: { check_updates: true } } },
      { name: 'update_settings', args: { patch: { check_updates: false } } },
    ]);
  });

  it('check_now_sends_check_for_updates', async () => {
    const bridge = fakeBridge(fixture('up-to-date'));
    const root = await render(bridge);
    button(root, 'Check now')!.click();
    flushSync();
    expect(bridge.calls).toEqual([{ name: 'check_for_updates', args: {} }]);

    // Negative controls: while a check runs, or an install, the button is
    // disabled and a click sends nothing.
    for (const name of ['checking', 'install-downloading', 'install-verifying', 'install-restarting']) {
      const busy = fakeBridge(fixture(name));
      const busyRoot = await render(busy);
      button(busyRoot, 'Check now')!.click();
      flushSync();
      expect(busy.calls, name).toEqual([]);
    }

    // After a failed install or with an unsigned release it stays.
    for (const name of ['install-failed', 'install-unsigned']) {
      const b = fakeBridge(fixture(name));
      const r = await render(b);
      button(r, 'Check now')!.click();
      flushSync();
      expect(b.calls, name).toEqual([{ name: 'check_for_updates', args: {} }]);
    }
  });

  it('install_button_sends_install_update', async () => {
    // `ready`: one primary button, labelled by the app.
    const ready = fixture('available');
    expect(ready.install.state).toBe('ready');
    const bridge = fakeBridge(ready);
    const root = await render(bridge);
    const install = button(root, ready.install.text)!;
    expect(ready.install.text).toBe('Install 1.3.0 and restart');
    expect(install).toBeDefined();
    expect(install.classList.contains('primary')).toBe(true);
    expect(install.disabled).toBe(false);
    install.click();
    flushSync();
    expect(bridge.calls).toEqual([{ name: 'install_update', args: {} }]);

    // A failed check keeps the newer version, and its install button.
    expect(fixture('failed').install.state).toBe('ready');
    const failedCheck = await render(fakeBridge(fixture('failed')));
    expect(button(failedCheck, 'Install 1.3.0 and restart')).toBeDefined();

    // While a check runs the install is refused, so the button is disabled.
    bridge.push({ ...ready, update: { ...ready.update, state: 'checking', text: 'Checking for updates…' } });
    expect(button(root, 'Install 1.3.0 and restart')!.disabled).toBe(true);

    // Negative control: in every other install state there is no install
    // button (the failed install's is "Try again").
    for (const name of PREVIEW_STATES) {
      const view = fixture(name);
      if (view.install.state === 'ready') continue;
      const r = await render(fakeBridge(view));
      expect(r.querySelector('button.install'), name).toBeNull();
      expect([...r.querySelectorAll('button')].filter((b) => /^Install/.test(text(b))), name).toEqual([]);
    }

    // The button goes when the next view has no newer version.
    bridge.push(fixture('up-to-date'));
    expect(root.querySelector('button.install')).toBeNull();
  });

  it('download_progress_is_shown_with_cancel', async () => {
    const view = fixture('install-downloading');
    expect(view.install.progress).toEqual({ bytes: 0, total: 40067 });
    const bridge = fakeBridge(view);
    const root = await render(bridge);

    // A determinate bar, its numbers in words beside it.
    const bar = root.querySelector('progress')!;
    expect(bar).not.toBeNull();
    expect(bar.getAttribute('max')).toBe('40067');
    expect(bar.getAttribute('value')).toBe('0');
    expect(bar.getAttribute('aria-label')).toBe('Download');
    expect(text(root.querySelector('.install-line'))).toContain('Downloading Crafting Helper 1.3.0…');
    expect(text(root.querySelector('.progress-text'))).toBe('0 KB of 39 KB');

    // The bar follows the next view.
    bridge.push({ ...view, install: { ...view.install, progress: { bytes: 20480, total: 40067 } } });
    expect(root.querySelector('progress')!.getAttribute('value')).toBe('20480');
    expect(text(root.querySelector('.progress-text'))).toBe('20 KB of 39 KB');

    // With no known total, the bar is indeterminate and the text says what came.
    bridge.push({ ...view, install: { ...view.install, progress: { bytes: 3 * 1048576 + 300000, total: null } } });
    expect(root.querySelector('progress')!.hasAttribute('value')).toBe(false);
    expect(text(root.querySelector('.progress-text'))).toBe('3.3 MB');

    // Cancel sends cancel_update; there is no install button and no Try again meanwhile.
    expect(button(root, 'Try again')).toBeUndefined();
    expect(root.querySelector('button.install')).toBeNull();
    button(root, 'Cancel')!.click();
    flushSync();
    expect(bridge.calls).toEqual([{ name: 'cancel_update', args: {} }]);

    // Negative controls: verifying and restarting show their line, no bar and no Cancel.
    for (const name of ['install-verifying', 'install-restarting']) {
      const v = fixture(name);
      const r = await render(fakeBridge(v));
      expect(r.querySelector('progress'), name).toBeNull();
      expect(button(r, 'Cancel'), name).toBeUndefined();
      expect(text(r.querySelector('.install-line')), name).toContain(v.install.text);
    }
    // A cancel goes back to ready: the bar goes, the install button is back.
    bridge.push(fixture('available'));
    expect(root.querySelector('progress')).toBeNull();
    expect(button(root, 'Cancel')).toBeUndefined();
    expect(button(root, 'Install 1.3.0 and restart')).toBeDefined();
  });

  it('failure_shows_the_reason_and_try_again_without_a_page', async () => {
    // A failed install: its one-line reason, marked in words, and Try again.
    const failed = fixture('install-failed');
    const bridge = fakeBridge(failed);
    const root = await render(bridge);
    const line = root.querySelector('.install-line')!;
    expect(line.getAttribute('data-install')).toBe('failed');
    expect(text(line)).toBe(`!${failed.install.text}`);
    expect(line.querySelector('.mark')?.getAttribute('aria-hidden')).toBe('true');
    expect(failed.install.text).toBe('Could not install Crafting Helper 1.3.0: the update is signed with another key.');
    button(root, 'Try again')!.click();
    flushSync();
    expect(bridge.calls).toEqual([{ name: 'install_update', args: {} }]);

    // An unsigned release: its line, and neither an install button nor Try again.
    const unsigned = fixture('install-unsigned');
    const r = await render(fakeBridge(unsigned));
    expect(text(r.querySelector('.install-line'))).toBe(unsigned.install.text);
    expect(unsigned.install.text).toBe('1.3.0 is available but cannot be installed: the release is not signed.');
    expect(button(r, 'Try again')).toBeUndefined();
    expect(r.querySelector('button.install')).toBeNull();

    // Try again only after a failed install.
    for (const name of PREVIEW_STATES) {
      const view = fixture(name);
      const page = await render(fakeBridge(view));
      expect(button(page, 'Try again') !== undefined, name).toBe(view.install.state === 'failed');

      // In no state does anything point at a release page, a browser or a URL.
      expect(page.querySelectorAll('a'), name).toHaveLength(0);
      for (const b of page.querySelectorAll('button')) {
        expect(text(b), name).not.toMatch(/release page|browser|github|download page|open/i);
      }
      expect(page.innerHTML, name).not.toMatch(/https?:|www\.|\.com\b/i);
    }
  });

  it('credit_reads_created_by_st4h', async () => {
    for (const name of ['idle', 'install-failed']) {
      const root = await render(fakeBridge(fixture(name)));
      const footer = root.querySelector('footer')!;
      expect(text(footer)).toBe('Created by ST4H');
      expect(footer.querySelector('a')).toBeNull();
      // The credit is the window's footer, outside its main region.
      expect(root.querySelector('main footer')).toBeNull();
      expect(root.querySelector('.window > footer')).toBe(footer);
    }

    // Before the first view arrives: the title and the credit, no section.
    const root = await render(fakeBridge(null), false);
    expect(text(root.querySelector('h1'))).toBe('Settings');
    expect(root.querySelector('section')).toBeNull();
    expect(text(root.querySelector('footer'))).toBe('Created by ST4H');
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
    const opened: string[] = [];
    vi.stubGlobal('open', (url: string) => {
      opened.push(String(url));
      return null;
    });

    // Positive control: the recorders do see a request made through each door.
    void fetch('probe').catch(() => {});
    new XMLHttpRequest().open('GET', 'probe');
    new WebSocket('probe');
    window.open('probe');
    expect(requests).toEqual(['fetch probe', 'xhr GET probe', 'ws probe']);
    expect(opened).toEqual(['probe']);
    requests.length = 0;
    opened.length = 0;

    // Every fixture rendered, through the app's bridge shape and the preview's,
    // and every control on it clicked.
    for (const name of PREVIEW_STATES) {
      for (const bridge of [fakeBridge(fixture(name)), previewSettingsBridge(`?state=${name}`)]) {
        vi.spyOn(console, 'info').mockImplementation(() => {});
        const root = await render(bridge);
        expect(root.querySelector('.window')?.getAttribute('data-install')).toBe(fixture(name).install.state);
        toggle(root).click();
        for (const b of root.querySelectorAll('button')) b.click();
        flushSync();
        await new Promise((resolve) => setTimeout(resolve, 0));
      }
    }
    expect(requests).toEqual([]);
    expect(opened).toEqual([]);
  });

  it('settings_problems_catch_drift', () => {
    for (const name of PREVIEW_STATES) expect(settingsProblems(fixture(name)), name).toEqual([]);

    // An extra key, at the top, inside `update` and inside `install`.
    const extra = fixture('idle') as unknown as Record<string, unknown>;
    extra.channel = 'beta';
    expect(settingsProblems(extra)).toEqual(['app_settings.channel: not in the TypeScript types']);
    const extraInner = fixture('idle') as unknown as { update: Record<string, unknown> };
    extraInner.update.url = 'x';
    expect(settingsProblems(extraInner)).toEqual(['app_settings.update.url: not in the TypeScript types']);
    const extraInstall = fixture('install-failed') as unknown as { install: Record<string, unknown> };
    extraInstall.install.browser_url = 'x';
    expect(settingsProblems(extraInstall)).toEqual(['app_settings.install.browser_url: not in the TypeScript types']);

    // A missing key.
    const missing = fixture('idle') as unknown as Record<string, unknown>;
    delete missing.check_updates;
    expect(settingsProblems(missing)).toEqual(['app_settings.check_updates: missing']);
    const missingInner = fixture('available') as unknown as { update: Record<string, unknown> };
    delete missingInner.update.newer;
    expect(settingsProblems(missingInner)).toEqual(['app_settings.update.newer: missing']);
    const missingProgress = fixture('install-downloading') as unknown as { install: { progress: Record<string, unknown> } };
    delete missingProgress.install.progress.total;
    expect(settingsProblems(missingProgress)).toEqual(['app_settings.install.progress.total: missing']);

    // A bad state, and a wrong type.
    const badState = fixture('idle') as unknown as { update: { state: string } };
    badState.update.state = 'up_to_date';
    expect(settingsProblems(badState)).toEqual([
      `app_settings.update.state: "up_to_date" is not one of ${UPDATE_STATES.join('|')}`,
    ]);
    const badInstall = fixture('available') as unknown as { install: { state: string } };
    badInstall.install.state = 'open-page';
    expect(settingsProblems(badInstall)).toEqual([
      `app_settings.install.state: "open-page" is not one of ${INSTALL_STATES.join('|')}`,
    ]);
    const badType = fixture('idle') as unknown as Record<string, unknown>;
    badType.check_updates = 'false';
    expect(settingsProblems(badType)).toEqual(['app_settings.check_updates: expected a boolean']);
    const badBytes = fixture('install-downloading') as unknown as { install: { progress: { bytes: unknown } } };
    badBytes.install.progress.bytes = -1;
    expect(settingsProblems(badBytes)).toEqual(['app_settings.install.progress.bytes: expected a whole number']);
    expect(settingsProblems(null)).toEqual(['app_settings: expected an object']);

    // The app's own fields: the hotkey, its status and the blocked list.
    const noHotkey = fixture('idle') as unknown as Record<string, unknown>;
    delete noHotkey.hotkey;
    expect(settingsProblems(noHotkey)).toEqual(['app_settings.hotkey: missing']);
    const badHotkeyStatus = fixture('hotkey-refused') as unknown as { hotkey_status: Record<string, unknown> };
    badHotkeyStatus.hotkey_status.kind = 'warn';
    expect(settingsProblems(badHotkeyStatus)).toEqual(['app_settings.hotkey_status.kind: "warn" is not one of info|error']);
    const badList = fixture('blocked-currencies') as unknown as { blocked_currencies: unknown[] };
    badList.blocked_currencies[1] = 7;
    expect(settingsProblems(badList)).toEqual(['app_settings.blocked_currencies[1]: expected a string']);
    const notList = fixture('idle') as unknown as Record<string, unknown>;
    notList.blocked_currencies = 'divine';
    expect(settingsProblems(notList)).toEqual(['app_settings.blocked_currencies: expected a list']);

    // The overlay's position and the screen watch.
    const badPosition = fixture('overlay-settings') as unknown as Record<string, unknown>;
    badPosition.overlay_position = 'center';
    badPosition.screen_watch = 'off';
    expect(settingsProblems(badPosition)).toEqual([
      'app_settings.overlay_position: "center" is not one of top-left|top-center|top-right|bottom-left|bottom-center|bottom-right',
      'app_settings.screen_watch: expected a boolean',
    ]);
    const noWatch = fixture('idle') as unknown as Record<string, unknown>;
    delete noWatch.screen_watch;
    expect(settingsProblems(noWatch)).toEqual(['app_settings.screen_watch: missing']);
  });

  it('hotkey_form_sends_set_hotkey', async () => {
    const bridge = fakeBridge(fixture('idle'));
    const root = await render(bridge);
    const region = root.querySelector('section[aria-labelledby="section-grab"]')!;
    expect(text(region.querySelector('h2'))).toBe('Grab');
    expect(text(region.querySelector('kbd.hotkey'))).toBe('Ctrl+Alt+X');
    const field = region.querySelector<HTMLInputElement>('#hotkey-field')!;
    const set = button(region as HTMLElement, 'Set')!;
    expect(document.querySelector(`label[for="${field.id}"]`)?.textContent).toBe('New hotkey');

    // Empty or blank: Set is disabled and a submit sends nothing.
    expect(set.disabled).toBe(true);
    setValue(field, '   ');
    expect(set.disabled).toBe(true);
    field.form!.requestSubmit();
    flushSync();
    expect(bridge.calls).toEqual([]);

    // A key, trimmed, by the button and by Enter in the field (a form submit).
    setValue(field, ' Ctrl+Alt+G ');
    expect(set.disabled).toBe(false);
    set.click();
    flushSync();
    expect(bridge.calls).toEqual([{ name: 'set_hotkey', args: { hotkey: 'Ctrl+Alt+G' } }]);
    expect(field.value).toBe('');
    setValue(field, 'Ctrl+Shift+F2');
    field.form!.requestSubmit();
    flushSync();
    expect(bridge.calls[1]).toEqual({ name: 'set_hotkey', args: { hotkey: 'Ctrl+Shift+F2' } });

    // The game's own Ctrl+Alt+C is sent too: the app refuses it, and the
    // section shows the refusal and the hotkey the app kept.
    setValue(field, 'ctrl+alt+c');
    set.click();
    flushSync();
    expect(bridge.calls[2]).toEqual({ name: 'set_hotkey', args: { hotkey: 'ctrl+alt+c' } });
    const refused = fixture('hotkey-refused');
    bridge.push(refused);
    const status = region.querySelector('#hotkey-status')!;
    expect(status.getAttribute('role')).toBe('status');
    expect(status.getAttribute('aria-live')).toBe('polite');
    expect(status.getAttribute('data-kind')).toBe('error');
    expect(text(status)).toBe(`!${refused.hotkey_status!.text}`);
    expect(text(region.querySelector('kbd.hotkey'))).toBe('Ctrl+Alt+X');
    expect(field.getAttribute('aria-describedby')).toContain('hotkey-status');

    // A changed hotkey shows once the next view carries it, and the status goes with it.
    bridge.push({ ...refused, hotkey: 'Ctrl+Alt+G', hotkey_status: { kind: 'info', text: 'Hotkey is now Ctrl+Alt+G.' } });
    expect(text(region.querySelector('kbd.hotkey'))).toBe('Ctrl+Alt+G');
    expect(text(status)).toBe('Hotkey is now Ctrl+Alt+G.');
    bridge.push(fixture('idle'));
    expect(text(status)).toBe('');
    expect(bridge.calls).toHaveLength(3);
  });

  it('settings_overlay_section_sends_position_and_watch', async () => {
    // Every other view has the defaults: top-center, the screen watch on.
    for (const name of PREVIEW_STATES.filter((n) => n !== 'overlay-settings')) {
      expect([fixture(name).overlay_position, fixture(name).screen_watch], name).toEqual(['top-center', true]);
    }
    const saved = fixture('overlay-settings');
    expect([saved.overlay_position, saved.screen_watch]).toEqual(['bottom-right', false]);
    const bridge = fakeBridge(saved);
    const root = await render(bridge);

    // After Grab, before the blocked list and Updates.
    expect(SECTIONS.map((s) => s.id)).toEqual(['grab', 'overlay', 'blocked', 'updates']);
    const region = overlaySection(root);
    expect(text(region.querySelector('h2'))).toBe('Overlay');

    // Position: a native select, labelled, the six places in reading order, the saved one selected.
    const select = region.querySelector<HTMLSelectElement>('select')!;
    expect(text(document.querySelector(`label[for="${select.id}"]`))).toBe('Position');
    expect([...select.options].map((o) => [o.value, o.textContent])).toEqual([
      ['top-left', 'Top left'],
      ['top-center', 'Top center'],
      ['top-right', 'Top right'],
      ['bottom-left', 'Bottom left'],
      ['bottom-center', 'Bottom center'],
      ['bottom-right', 'Bottom right'],
    ]);
    expect(select.value).toBe('bottom-right');
    expect(text(document.getElementById(select.getAttribute('aria-describedby')!))).toBe(
      "Where the step panel sits in the game's window while a craft is guided.",
    );

    // The screen watch: a switch, named by its label, described by its help, off as saved.
    const watch = region.querySelector<HTMLInputElement>('input[role="switch"]')!;
    expect(text(watch.closest('label'))).toBe('Read the item from the screen');
    expect(watch.checked).toBe(false);
    expect(text(document.getElementById(watch.getAttribute('aria-describedby')!))).toBe(
      'After each step, hover the item in the game and the helper reads it. Copies of the screen stay in memory; nothing is saved or sent. Off: grab the item after each step.',
    );

    // Each control sends its one key.
    choose(select, 'top-left');
    expect(bridge.calls).toEqual([{ name: 'update_settings', args: { patch: { overlay_position: 'top-left' } } }]);
    watch.click();
    flushSync();
    expect(bridge.calls[1]).toEqual({ name: 'update_settings', args: { patch: { screen_watch: true } } });

    // Both follow the next view, not their own change.
    bridge.push(fixture('idle'));
    expect(select.value).toBe('top-center');
    expect(watch.checked).toBe(true);
    bridge.push(saved);
    expect(select.value).toBe('bottom-right');
    expect(watch.checked).toBe(false);
    watch.click();
    flushSync();
    expect(bridge.calls[2]).toEqual({ name: 'update_settings', args: { patch: { screen_watch: true } } });
    expect(bridge.calls).toHaveLength(3);

    // The Updates switch is a different control and sends its own key.
    toggle(root).click();
    flushSync();
    expect(bridge.calls[3]).toEqual({ name: 'update_settings', args: { patch: { check_updates: true } } });
  });

  it('blocked_currencies_picker_offers_only_catalogue_names', async () => {
    const saved = fixture('blocked-currencies');
    expect(saved.blocked_currencies).toEqual(['divine-orb', 'chaos-orb', 'exalted-orb']);
    const bridge = fakeBridge(saved);
    const root = await render(bridge);
    const region = root.querySelector<HTMLElement>('section[aria-labelledby="section-blocked"]')!;
    expect(text(region.querySelector('h2'))).toBe('Blocked currencies');
    expect(text(region.querySelector('.help'))).toBe('Currencies the helper never suggests. Pick them by name.');
    expect(text(region.querySelector('.count'))).toBe('3 blocked');

    // The window asked for the catalogue once.
    expect(bridge.catalogueCalls).toBe(1);

    // No field takes a typed id: the only text field is the picker's filter,
    // and there is no Add or Remove.
    expect(region.querySelector('#blocked-field')).toBeNull();
    expect([...region.querySelectorAll('input:not([type="checkbox"])')].map((el) => el.getAttribute('aria-label'))).toEqual([
      'Filter currencies',
    ]);
    expect(button(region, 'Add')).toBeUndefined();
    expect(region.querySelector('.remove')).toBeNull();

    // The picker offers the catalogue and nothing else, each entry by its
    // name, in its groups; with no tag in Settings.
    const boxes = () => [...region.querySelectorAll<HTMLInputElement>('input[type="checkbox"][data-id]')];
    const box = (id: string) => region.querySelector<HTMLInputElement>(`input[type="checkbox"][data-id="${id}"]`)!;
    const entries = CATALOGUE.flatMap((group) => group.entries);
    expect(entries.length).toBeGreaterThan(3);
    expect(boxes().map((b) => b.dataset.id)).toEqual(entries.map((entry) => entry.id));
    expect(boxes().map((b) => text(b.closest('label')))).toEqual(entries.map((entry) => entry.name));
    expect([...region.querySelectorAll('legend')].map((el) => text(el))).toEqual(CATALOGUE.map((group) => group.label));
    expect(region.querySelector('.tag')).toBeNull();

    // A saved id is shown by its name, checked; its id is not shown anywhere.
    const nameOf = (id: string) => entries.find((entry) => entry.id === id)!.name;
    const checkedNames = () => boxes().filter((b) => b.checked).map((b) => text(b.closest('label')));
    expect(checkedNames().sort()).toEqual(saved.blocked_currencies.map(nameOf).sort());
    for (const id of saved.blocked_currencies) expect(text(region)).not.toContain(id);

    // Picking sends the whole list with the new id last; the box shows the
    // view, so it stays unchecked until the next view carries the list.
    const fresh = entries.find((entry) => !saved.blocked_currencies.includes(entry.id))!;
    click(box(fresh.id));
    expect(bridge.calls).toEqual([
      { name: 'update_settings', args: { patch: { blocked_currencies: [...saved.blocked_currencies, fresh.id] } } },
    ]);
    expect(box(fresh.id).checked).toBe(false);
    bridge.push({ ...saved, blocked_currencies: [...saved.blocked_currencies, fresh.id] });
    expect(box(fresh.id).checked).toBe(true);
    expect(text(region.querySelector('.count'))).toBe('4 blocked');

    // Unpicking sends the whole list without that id.
    click(box('chaos-orb'));
    expect(bridge.calls[1]).toEqual({
      name: 'update_settings',
      args: { patch: { blocked_currencies: ['divine-orb', 'exalted-orb', fresh.id] } },
    });

    // The filter narrows the list by name; no match says so and sends nothing.
    const filter = region.querySelector<HTMLInputElement>('input[type="search"]')!;
    setValue(filter, 'orb');
    const shown = boxes().filter((b) => !b.closest('li')!.hidden);
    expect(shown.length).toBeGreaterThan(0);
    expect(shown.length).toBeLessThan(entries.length);
    for (const b of shown) expect(text(b.closest('label')).toLowerCase()).toContain('orb');
    setValue(filter, 'no such currency');
    expect(text(region.querySelector('.no-match'))).toBe('No currency matches “no such currency”.');
    expect(bridge.calls).toHaveLength(2);

    // Negative control: an id the catalogue does not hold is never offered,
    // even if a view carried one; only the catalogue's names are.
    bridge.push({ ...saved, blocked_currencies: ['not-a-currency', 'chaos-orb'] });
    expect(region.querySelector('input[data-id="not-a-currency"]')).toBeNull();
    expect(text(region)).not.toContain('not-a-currency');
    expect(boxes().map((b) => b.dataset.id)).toEqual(entries.map((entry) => entry.id));
    expect(bridge.catalogueCalls).toBe(1);

    // An empty list says so; the first pick sends a list of one, and
    // unpicking the last sends an empty list.
    const empty = fakeBridge(fixture('idle'));
    const emptyRoot = await render(empty);
    const emptyRegion = emptyRoot.querySelector<HTMLElement>('section[aria-labelledby="section-blocked"]')!;
    expect(text(emptyRegion.querySelector('.count'))).toBe('No currency is blocked.');
    click(emptyRegion.querySelector('input[data-id="chaos-orb"]'));
    expect(empty.calls).toEqual([{ name: 'update_settings', args: { patch: { blocked_currencies: ['chaos-orb'] } } }]);
    empty.push({ ...fixture('idle'), blocked_currencies: ['chaos-orb'] });
    expect(text(emptyRegion.querySelector('.count'))).toBe('1 blocked');
    click(emptyRegion.querySelector('input[data-id="chaos-orb"]'));
    expect(empty.calls[1]).toEqual({ name: 'update_settings', args: { patch: { blocked_currencies: [] } } });
  });

  it('tauri_bridge_subscribes_then_asks', async () => {
    const asked: string[] = [];
    mockIPC(
      (cmd, payload) => {
        asked.push(cmd);
        if (cmd === 'get_app_settings') return fixture('idle');
        if (cmd === 'get_catalogue') return CATALOGUE;
        if (cmd === 'install_update') {
          expect(payload).toEqual({});
          return fixture('install-downloading');
        }
        return undefined;
      },
      { shouldMockEvents: true },
    );
    const views: SettingsView[] = [];
    const bridge = tauriSettingsBridge();
    const off = await bridge.start((v) => views.push(v));
    expect(asked).toContain('get_app_settings');
    expect(views.map((v) => v.install.state)).toEqual(['none']);

    // A view the shell sends on `ch:app-settings` is rendered ...
    expect(APP_SETTINGS_EVENT).toBe('ch:app-settings');
    await emit(APP_SETTINGS_EVENT, fixture('available'));
    expect(views.map((v) => v.install.state)).toEqual(['none', 'ready']);

    // ... and a command's own answer is not (the events carry every change).
    await bridge.command('install_update', {});
    expect(asked).toContain('install_update');
    expect(views).toHaveLength(2);

    off();
    await emit(APP_SETTINGS_EVENT, fixture('install-failed'));
    expect(views).toHaveLength(2);

    // The catalogue the blocked list picks from is the app's `get_catalogue`.
    expect(await bridge.catalogue()).toEqual(CATALOGUE);
    expect(asked).toContain('get_catalogue');
  });

  it('preview_reads_the_state_from_the_query', async () => {
    expect(parseSettingsPreviewQuery('')).toEqual({ state: 'idle', scale: null });
    expect(parseSettingsPreviewQuery('?state=install-failed&scale=2')).toEqual({ state: 'install-failed', scale: '2' });
    expect(parseSettingsPreviewQuery('?state=sideways&scale=3')).toEqual({ state: 'idle', scale: null });

    const root = await render(previewSettingsBridge('?state=install-downloading&scale=2'));
    expect(root.querySelector('.window')?.getAttribute('data-install')).toBe('downloading');
    expect(document.documentElement.dataset.previewScale).toBe('2');

    // The preview's picker reads the catalogue fixture.
    expect(await previewSettingsBridge('').catalogue()).toEqual(CATALOGUE);
    expect(root.querySelectorAll('section[aria-labelledby="section-blocked"] input[data-id]')).toHaveLength(
      CATALOGUE.flatMap((group) => group.entries).length,
    );
  });
});
