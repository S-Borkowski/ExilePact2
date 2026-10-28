// Shared by the pages' tests (`*.test.ts`): the view fixtures under
// `fixtures/views/` and `fixtures/app-settings/`, fake bridges that deliver
// them as the shell would and record every command, and mount helpers.
// Never imported by the app.

import { type Component, flushSync, mount, unmount } from 'svelte';
import App from './App.svelte';
import type { Bridge, Command, CommandArgs } from './bridge';
import type { View } from './ipc';
import type { SettingsBridge, SettingsCommand, SettingsCommandArgs } from './settings/bridge';
import type { SettingsView } from './settings/view';

const byName = (raw: Record<string, unknown>): Record<string, unknown> =>
  Object.fromEntries(Object.entries(raw).map(([path, json]) => [path.replace(/^.*\/([^/]+)\.json$/, '$1'), json]));

/** Each main-view fixture by name (`empty`, `grabbed-item`, ...), as parsed JSON. */
export const FIXTURES: Record<string, unknown> = byName(
  import.meta.glob('../fixtures/views/*.json', { eager: true, import: 'default' }) as Record<string, unknown>,
);

/** The states the context's Fixtures list names for the main view, one file each. */
export const FIXTURE_NAMES = ['empty', 'grabbed-item', 'grabbed-not-item', 'grab-refused', 'hotkey-taken', 'overlay-active'] as const;

/** Each Settings fixture by name (`idle`, `hotkey-refused`, ...), as parsed JSON. */
export const SETTINGS_FIXTURES: Record<string, unknown> = byName(
  import.meta.glob('../fixtures/app-settings/*.json', { eager: true, import: 'default' }) as Record<string, unknown>,
);

/** A fresh copy of a main-view fixture. */
export function fixtureView(name: string): View {
  const json = FIXTURES[name];
  if (json === undefined) throw new Error(`no fixture views/${name}.json`);
  return JSON.parse(JSON.stringify(json)) as View;
}

/** A fresh copy of a Settings fixture. */
export function settingsFixture(name: string): SettingsView {
  const json = SETTINGS_FIXTURES[name];
  if (json === undefined) throw new Error(`no fixture app-settings/${name}.json`);
  return JSON.parse(JSON.stringify(json)) as SettingsView;
}

export interface FakeBridge extends Bridge {
  calls: Array<{ name: Command; args: unknown }>;
  pushView(view: View): void;
}

/** A main-view bridge that delivers `initial` as the shell would and records every command. */
export function fakeBridge(initial: View): FakeBridge {
  let onView: ((view: View) => void) | null = null;
  const calls: Array<{ name: Command; args: unknown }> = [];
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
    async command<C extends Command>(name: C, args: CommandArgs[C]) {
      calls.push({ name, args });
    },
    pushView(view) {
      onView?.(view);
      flushSync();
    },
  };
}

export interface FakeSettingsBridge extends SettingsBridge {
  calls: Array<{ name: SettingsCommand; args: unknown }>;
  push(view: SettingsView): void;
}

/** A Settings bridge that delivers views as the shell would and records every command; `null` delivers none. */
export function fakeSettingsBridge(initial: SettingsView | null): FakeSettingsBridge {
  let onView: ((view: SettingsView) => void) | null = null;
  const calls: Array<{ name: SettingsCommand; args: unknown }> = [];
  return {
    kind: 'tauri',
    calls,
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

export interface Rendered {
  root: HTMLElement;
  destroy(): void;
}

/**
 * Mounts `component` with `props` and waits until `ready` finds what the
 * first view draws (at most a second).
 */
export async function renderComponent<P extends Record<string, unknown>>(
  component: Component<P>,
  props: P,
  ready: (root: HTMLElement) => boolean,
): Promise<Rendered> {
  const target = document.createElement('div');
  document.body.appendChild(target);
  const app = mount(component, { target, props });
  flushSync();
  for (let i = 0; i < 200 && !ready(target); i++) {
    await new Promise((resolve) => setTimeout(resolve, 5));
    flushSync();
  }
  return {
    root: target,
    destroy() {
      unmount(app);
      target.remove();
    },
  };
}

/** Mounts the main window on a fake bridge serving `view` and lets it take that view. */
export async function renderView(view: View): Promise<Rendered & { bridge: FakeBridge }> {
  const bridge = fakeBridge(view);
  const rendered = await renderComponent(App, { bridge }, (root) => root.querySelector('.app[data-state]') !== null);
  return { ...rendered, bridge };
}

/** Text with its whitespace runs folded to one space, trimmed. */
export const text = (el: Element | null | undefined): string => (el?.textContent ?? '').replace(/\s+/g, ' ').trim();

/** The button whose text is `label`. */
export const button = (root: Element, label: string): HTMLButtonElement | undefined =>
  [...root.querySelectorAll('button')].find((b) => text(b) === label);

/** Sets a text field's value as typing would, then fires its events. */
export function setValue(el: HTMLInputElement, value: string): void {
  el.value = value;
  el.dispatchEvent(new Event('input', { bubbles: true }));
  el.dispatchEvent(new Event('change', { bubbles: true }));
  flushSync();
}

/** Clicks an element and lets Svelte apply what it changed. */
export function click(el: Element | null | undefined): void {
  if (!el) throw new Error('nothing to click');
  (el as HTMLElement).click();
  flushSync();
}

/**
 * Fires a paste at `target` as Ctrl+V would: a bubbling `paste` event whose
 * `clipboardData` holds `text` as plain text (`null`: no clipboard data at
 * all). jsdom has no `ClipboardEvent` or `DataTransfer`, so the data is a
 * stand-in with the one method the page reads.
 */
export function paste(target: EventTarget, text: string | null): Event {
  const event = new Event('paste', { bubbles: true, cancelable: true });
  const data = text === null ? null : { getData: (type: string) => (type === 'text/plain' || type === 'text' ? text : '') };
  Object.defineProperty(event, 'clipboardData', { value: data });
  target.dispatchEvent(event);
  flushSync();
  return event;
}

/**
 * A file read from disk, `path` relative to this folder (`src/`), as bytes.
 * `node:fs` is reached by a dynamic import: the page's TypeScript config
 * carries no Node types, and the app never imports it.
 */
export async function readBytes(path: string): Promise<Uint8Array> {
  const fs = (await import(/* @vite-ignore */ 'node:fs' as string)) as { readFileSync(file: URL): Uint8Array };
  return fs.readFileSync(new URL(path, import.meta.url));
}

/** The file names in a folder, `path` relative to `src/`, sorted. */
export async function listFiles(path: string): Promise<string[]> {
  const fs = (await import(/* @vite-ignore */ 'node:fs' as string)) as { readdirSync(dir: URL): string[] };
  return fs.readdirSync(new URL(path, import.meta.url)).sort();
}
