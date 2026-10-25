// Shared by the UI tests (`*.test.ts`): the serialised views the Rust side
// writes under `fixtures/viewmodels/`, a fake bridge that delivers them as
// the shell would, and a mount helper. Never imported by the app.

import { flushSync, mount, unmount } from 'svelte';
import App from './App.svelte';
import type { Bridge, Command, CommandArgs } from './bridge';
import type { Mode, Toast, View } from './ipc';

const RAW = import.meta.glob('../fixtures/viewmodels/*.json', { eager: true, import: 'default' }) as Record<string, unknown>;

/** Each fixture file by name (`panel-campaign`, `toast`, ...), as parsed JSON. */
export const FIXTURES: Record<string, unknown> = Object.fromEntries(
  Object.entries(RAW).map(([path, json]) => [path.replace(/^.*\/([^/]+)\.json$/, '$1'), json]),
);

/** A fresh copy of a fixture view, optionally in another mode. */
export function fixtureView(name: string, mode?: Mode): View {
  const json = FIXTURES[name];
  if (json === undefined) throw new Error(`no fixture ${name}.json`);
  const view = JSON.parse(JSON.stringify(json)) as View;
  if (mode) view.mode = mode;
  return view;
}

export function fixtureToast(): Toast {
  return JSON.parse(JSON.stringify(FIXTURES.toast)) as Toast;
}

export interface FakeBridge extends Bridge {
  calls: Array<{ name: Command; args: unknown }>;
  pushView(view: View): void;
  pushToast(toast: Toast): void;
}

export function fakeBridge(initial: View | null, holdToasts = false): FakeBridge {
  let onView: ((view: View) => void) | null = null;
  let onToast: ((toast: Toast) => void) | null = null;
  const calls: Array<{ name: Command; args: unknown }> = [];
  return {
    kind: 'tauri',
    holdToasts,
    calls,
    async start(v, t) {
      onView = v;
      onToast = t;
      if (initial) v(initial);
      return () => {
        onView = null;
        onToast = null;
      };
    },
    async command<C extends Command>(name: C, args: CommandArgs[C]) {
      calls.push({ name, args });
    },
    pushView(view) {
      onView?.(view);
      flushSync();
    },
    pushToast(toast) {
      onToast?.(toast);
      flushSync();
    },
  };
}

export interface Rendered {
  root: HTMLElement;
  bridge: Bridge;
  destroy(): void;
}

/** Mounts the app on `bridge` and lets it take its first view. */
export async function renderApp(bridge: Bridge): Promise<Rendered> {
  const target = document.createElement('div');
  document.body.appendChild(target);
  const app = mount(App, { target, props: { bridge } });
  flushSync();
  // `start` is async (the preview imports its fixture): wait until a view
  // has arrived, then render what it delivered.
  for (let i = 0; i < 200 && target.querySelector('.overlay')?.getAttribute('data-frame') === 'none'; i++) {
    await new Promise((resolve) => setTimeout(resolve, 5));
    flushSync();
  }
  await new Promise((resolve) => setTimeout(resolve, 0));
  flushSync();
  return {
    root: target,
    bridge,
    destroy() {
      unmount(app);
      target.remove();
    },
  };
}

/**
 * A file's text read from disk, `path` relative to this folder (`src/`).
 * Vitest blanks CSS imports, `?raw` ones too, so the CSS the tests check is
 * read this way. `node:fs` is reached by a dynamic import: the UI's
 * TypeScript config carries no Node types, and the app never imports it.
 */
export async function readText(path: string): Promise<string> {
  const fs = (await import(/* @vite-ignore */ 'node:fs' as string)) as {
    readFileSync(file: URL, encoding: 'utf8'): string;
  };
  return fs.readFileSync(new URL(path, import.meta.url), 'utf8');
}

/** The interactive elements play mode must not have. */
export const CONTROL_SELECTOR = 'button, input, select, textarea, a[href], [role="button"], [tabindex]';
