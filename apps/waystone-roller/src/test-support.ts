// Shared by the page's tests (`*.test.ts`): the view fixtures under
// `fixtures/views/`, a fake bridge that delivers them as the shell would and
// records every command, and a mount helper. Never imported by the app.

import { flushSync, mount, unmount } from 'svelte';
import App from './App.svelte';
import type { Bridge, Command, CommandArgs } from './bridge';
import type { View } from './ipc';

const RAW = import.meta.glob('../fixtures/views/*.json', { eager: true, import: 'default' }) as Record<string, unknown>;

/** Each fixture file by name (`empty`, `ok-any`, ...), as parsed JSON. */
export const FIXTURES: Record<string, unknown> = Object.fromEntries(
  Object.entries(RAW).map(([path, json]) => [path.replace(/^.*\/([^/]+)\.json$/, '$1'), json]),
);

/** The states the context's IPC contract lists, one fixture each. */
export const FIXTURE_NAMES = ['empty', 'ok-any', 'ok-all', 'too-long', 'unmatchable', 'hotkey-taken'] as const;

/** A fresh copy of a fixture view. */
export function fixtureView(name: string): View {
  const json = FIXTURES[name];
  if (json === undefined) throw new Error(`no fixture ${name}.json`);
  return JSON.parse(JSON.stringify(json)) as View;
}

export interface FakeBridge extends Bridge {
  calls: Array<{ name: Command; args: unknown }>;
  pushView(view: View): void;
}

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

export interface Rendered {
  root: HTMLElement;
  bridge: FakeBridge;
  destroy(): void;
}

/** Mounts the page on a fake bridge serving `view` and lets it take that view. */
export async function renderView(view: View): Promise<Rendered> {
  const bridge = fakeBridge(view);
  const target = document.createElement('div');
  document.body.appendChild(target);
  const app = mount(App, { target, props: { bridge } });
  flushSync();
  for (let i = 0; i < 200 && !target.querySelector('.app[data-state]'); i++) {
    await new Promise((resolve) => setTimeout(resolve, 5));
    flushSync();
  }
  return {
    root: target,
    bridge,
    destroy() {
      unmount(app);
      target.remove();
    },
  };
}

/** Sets a text field's or select's value as typing or choosing would, then fires its events. */
export function setValue(el: HTMLInputElement | HTMLSelectElement, value: string): void {
  el.value = value;
  el.dispatchEvent(new Event('input', { bubbles: true }));
  el.dispatchEvent(new Event('change', { bubbles: true }));
  flushSync();
}

/** Clicks an element and lets Svelte apply what it changed. */
export function click(el: Element | null): void {
  if (!el) throw new Error('nothing to click');
  (el as HTMLElement).click();
  flushSync();
}

/** A row of the mod list by mod id. */
export function modRow(root: Element, id: string): HTMLElement {
  const row = root.querySelector<HTMLElement>(`.mod[data-mod-id="${id}"]`);
  if (!row) throw new Error(`no row for ${id}`);
  return row;
}

/** One of a row's three choices: `want`, `none` or `avoid`. */
export function choice(root: Element, id: string, value: 'want' | 'none' | 'avoid'): HTMLInputElement {
  const input = modRow(root, id).querySelector<HTMLInputElement>(`input[type="radio"][value="${value}"]`);
  if (!input) throw new Error(`no ${value} choice on ${id}`);
  return input;
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
