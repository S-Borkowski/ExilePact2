// Measures, in a real Chromium layout, that the main window never scrolls
// past its content: for every view fixture under `fixtures/views/` (the
// preview's `?state=`) at the window's default size (960x680) and at a small
// one (600x480), the document must not scroll at all (its scroll height is
// the window's height) and no scroll pane may scroll further than the bottom
// of its own laid-out content plus its padding.
//
// jsdom does no layout, so this drives headless Edge (Chromium, the engine
// WebView2 runs) over the DevTools protocol with Node's own `WebSocket`, and
// serves the page with the installed Vite's dev server (the preview reads the
// fixtures only there). No package beyond the app's own is used.
//
//   node apps/crafting-helper/scripts/scroll-height.mjs
//
// Prints each failing state, size and pixel excess and exits 1 on any; exits
// 2, never 0, when Edge or the page cannot start. Edge is found at its usual
// install paths, or at `MSEDGE` when set.

import { spawn, spawnSync } from 'node:child_process';
import { existsSync, mkdtempSync, readdirSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createServer as netServer } from 'node:net';
import { createServer } from 'vite';

const APP = join(dirname(fileURLToPath(import.meta.url)), '..');
const SIZES = [
  { width: 960, height: 680 },
  { width: 600, height: 480 },
];
/** Sub-pixel rounding between a box's fractional bottom and an integer scroll height. */
const TOLERANCE = 1;
const START_TIMEOUT_MS = 30_000;
const RENDER_TIMEOUT_MS = 15_000;

class StartFailure extends Error {}

function edgePath() {
  const candidates = [
    process.env.MSEDGE,
    'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe',
    'C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe',
  ].filter(Boolean);
  const found = candidates.find((path) => existsSync(path));
  if (!found) throw new StartFailure(`Edge not found (looked at ${candidates.join(', ')}; set MSEDGE)`);
  return found;
}

/** A port nothing listens on right now, from the OS. */
function freePort() {
  return new Promise((resolve, reject) => {
    const server = netServer();
    server.unref();
    server.on('error', reject);
    server.listen(0, '127.0.0.1', () => {
      const { port } = server.address();
      server.close(() => resolve(port));
    });
  });
}

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

async function until(what, timeoutMs, probe) {
  const deadline = Date.now() + timeoutMs;
  for (;;) {
    const value = await probe();
    if (value) return value;
    if (Date.now() > deadline) throw new StartFailure(`timed out after ${timeoutMs} ms waiting for ${what}`);
    await sleep(100);
  }
}

/** One DevTools connection to the browser; commands carry a session id once attached to the page. */
async function devtools(url) {
  const socket = new WebSocket(url);
  await new Promise((resolve, reject) => {
    socket.addEventListener('open', resolve, { once: true });
    socket.addEventListener('error', () => reject(new StartFailure(`DevTools socket ${url} failed to open`)), { once: true });
  });
  let next = 0;
  const pending = new Map();
  const listeners = new Set();
  socket.addEventListener('message', (event) => {
    const message = JSON.parse(String(event.data));
    if (message.id !== undefined && pending.has(message.id)) {
      const { resolve, reject } = pending.get(message.id);
      pending.delete(message.id);
      if (message.error) reject(new Error(`${message.error.message} (${message.error.code})`));
      else resolve(message.result);
    } else {
      for (const listener of listeners) listener(message);
    }
  });
  return {
    send(method, params = {}, sessionId) {
      const id = ++next;
      socket.send(JSON.stringify({ id, method, params, ...(sessionId ? { sessionId } : {}) }));
      return new Promise((resolve, reject) => pending.set(id, { resolve, reject }));
    },
    on(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    close() {
      socket.close();
    },
  };
}

// Runs in the page: the document's and every scroll pane's excess, in px.
function measureInPage() {
  const out = [];
  const root = document.scrollingElement ?? document.documentElement;
  const describe = (el) =>
    el.tagName.toLowerCase() +
    (el.id ? `#${el.id}` : '') +
    Array.from(el.classList)
      .filter((name) => !name.startsWith('svelte-'))
      .map((name) => `.${name}`)
      .join('');
  const path = (el) => {
    const parts = [];
    for (let at = el; at && at !== document.body; at = at.parentElement) parts.unshift(describe(at));
    return parts.slice(-3).join(' > ');
  };
  // The box an element is laid out against: the nearest positioned ancestor
  // for an absolute one, the viewport for a fixed one, else its parent.
  const containingBlock = (el) => {
    const position = getComputedStyle(el).position;
    if (position === 'fixed') return null;
    if (position !== 'absolute') return el.parentElement;
    for (let at = el.parentElement; at; at = at.parentElement) {
      if (getComputedStyle(at).position !== 'static') return at;
    }
    return document.documentElement;
  };
  // Whether a box adds to a container's scrollable overflow: its chain of
  // containing blocks reaches the container before any box that clips.
  const reaches = (el, container) => {
    for (let at = containingBlock(el); at; at = containingBlock(at)) {
      if (at === container) return true;
      if (getComputedStyle(at).overflowY !== 'visible') return false;
    }
    return false;
  };
  // The lowest box that adds to a container's overflow, so a failure names its cause.
  const lowest = (container, scope, top) => {
    let found = null;
    for (const el of scope.querySelectorAll('*')) {
      if (!reaches(el, container)) continue;
      const bottom = el.getBoundingClientRect().bottom - top;
      if (!found || bottom > found.bottom) found = { el, bottom };
    }
    return found ? `${path(found.el)} (${getComputedStyle(found.el).position}) reaches ${Math.round(found.bottom)} px` : 'nothing';
  };
  // A pane's in-flow children, looking through `display: contents` boxes.
  const flowChildren = (el) =>
    Array.from(el.childNodes).flatMap((node) =>
      node.nodeType === Node.ELEMENT_NODE && getComputedStyle(node).display === 'contents' ? flowChildren(node) : [node],
    );

  const docExcess = root.scrollHeight - window.innerHeight;
  if (docExcess > 1) {
    out.push({ where: 'document', excess: docExcess, cause: lowest(document.documentElement, document.body, -window.scrollY) });
  }
  for (const pane of document.querySelectorAll('*')) {
    const style = getComputedStyle(pane);
    if (!['auto', 'scroll'].includes(style.overflowY)) continue;
    if (pane === document.documentElement || pane === document.body) continue;
    // A field scrolls its own text, not laid-out children.
    if (['TEXTAREA', 'SELECT', 'INPUT'].includes(pane.tagName)) continue;
    const box = pane.getBoundingClientRect();
    const top = box.top + parseFloat(style.borderTopWidth) - pane.scrollTop;
    // The bottom of the pane's in-flow content: its children with their
    // bottom margins, and its own text.
    let bottom = 0;
    for (const node of flowChildren(pane)) {
      let edge = 0;
      if (node.nodeType === Node.ELEMENT_NODE) {
        const childStyle = getComputedStyle(node);
        if (childStyle.display === 'none' || ['absolute', 'fixed'].includes(childStyle.position)) continue;
        edge = node.getBoundingClientRect().bottom + Math.max(0, parseFloat(childStyle.marginBottom) || 0);
      } else if (node.nodeType === Node.TEXT_NODE && node.textContent.trim()) {
        const range = document.createRange();
        range.selectNodeContents(node);
        edge = range.getBoundingClientRect().bottom;
      } else continue;
      bottom = Math.max(bottom, edge - top);
    }
    const allowed = Math.max(pane.clientHeight, bottom + parseFloat(style.paddingBottom));
    const excess = pane.scrollHeight - allowed;
    if (excess > 1) out.push({ where: path(pane), excess: Math.round(excess), cause: lowest(pane, pane, top) });
  }
  return out;
}

async function main() {
  const states = readdirSync(join(APP, 'fixtures', 'views'))
    .filter((name) => name.endsWith('.json'))
    .map((name) => name.slice(0, -'.json'.length))
    .sort();
  if (states.length === 0) throw new StartFailure('no view fixtures under fixtures/views/');

  const cleanups = [];
  try {
    const port = await freePort();
    const vite = await createServer({
      root: APP,
      configFile: join(APP, 'vite.config.ts'),
      logLevel: 'error',
      clearScreen: false,
      server: { port, strictPort: true, host: '127.0.0.1', hmr: false },
    });
    cleanups.push(() => vite.close());
    await vite.listen();
    const origin = `http://127.0.0.1:${port}`;

    const profile = mkdtempSync(join(tmpdir(), 'ch-scroll-height-'));
    cleanups.push(() => removeProfile(profile));
    const edge = spawn(
      edgePath(),
      [
        '--headless=new',
        '--remote-debugging-port=0',
        `--user-data-dir=${profile}`,
        '--no-first-run',
        '--no-default-browser-check',
        '--disable-extensions',
        '--disable-gpu',
        '--hide-scrollbars=false',
        'about:blank',
      ],
      { stdio: 'ignore' },
    );
    let edgeExit = null;
    edge.on('exit', (code) => (edgeExit = code));
    edge.on('error', (error) => (edgeExit = String(error)));
    cleanups.push(() => killTree(edge));

    // Edge picks its DevTools port and writes it to the profile.
    const activePort = join(profile, 'DevToolsActivePort');
    const [devtoolsPort, browserPath] = await until('Edge to open its DevTools port', START_TIMEOUT_MS, () => {
      if (edgeExit !== null) throw new StartFailure(`Edge exited early (${edgeExit})`);
      if (!existsSync(activePort)) return null;
      const lines = readFileSync(activePort, 'utf8').split(/\r?\n/);
      return lines.length >= 2 && lines[1] ? lines : null;
    });
    const cdp = await devtools(`ws://127.0.0.1:${devtoolsPort}${browserPath}`);
    cleanups.push(() => cdp.close());

    const { targetId } = await cdp.send('Target.createTarget', { url: 'about:blank' });
    const { sessionId } = await cdp.send('Target.attachToTarget', { targetId, flatten: true });
    const page = (method, params) => cdp.send(method, params, sessionId);
    const errors = [];
    cdp.on((message) => {
      if (message.sessionId !== sessionId) return;
      if (message.method === 'Runtime.exceptionThrown') {
        errors.push(message.params.exceptionDetails.exception?.description ?? message.params.exceptionDetails.text);
      }
    });
    await page('Page.enable');
    await page('Runtime.enable');

    const evaluate = async (expression) => {
      const result = await page('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
      if (result.exceptionDetails) throw new Error(result.exceptionDetails.exception?.description ?? result.exceptionDetails.text);
      return result.result.value;
    };

    const failures = [];
    let measured = 0;
    for (const size of SIZES) {
      await page('Emulation.setDeviceMetricsOverride', { ...size, deviceScaleFactor: 1, mobile: false });
      for (const state of states) {
        errors.length = 0;
        await page('Page.navigate', { url: `${origin}/?state=${encodeURIComponent(state)}` });
        await until(`the page to render ?state=${state}`, RENDER_TIMEOUT_MS, async () => {
          if (errors.length) throw new StartFailure(`the page threw on ?state=${state}: ${errors[0]}`);
          return evaluate(
            `document.readyState === 'complete' && location.search === ${JSON.stringify(`?state=${encodeURIComponent(state)}`)} && !!document.querySelector('#app .app[data-mode]')`,
          ).catch(() => false);
        });
        // Fonts in, then two frames for the layout to settle.
        await evaluate(
          'document.fonts.ready.then(() => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))))',
        );
        const inner = await evaluate('[window.innerWidth, window.innerHeight]');
        if (inner[0] !== size.width || inner[1] !== size.height) {
          throw new StartFailure(`the window is ${inner.join('x')}, not ${size.width}x${size.height}`);
        }
        const excesses = await evaluate(`(${measureInPage.toString()})()`);
        measured += 1;
        for (const { where, excess, cause } of excesses) {
          failures.push(`${state} ${size.width}x${size.height}: ${where} scrolls ${excess} px past its content; lowest: ${cause}`);
        }
      }
    }

    if (measured !== states.length * SIZES.length) throw new StartFailure(`measured ${measured} pages, expected ${states.length * SIZES.length}`);
    for (const line of failures) console.log(`FAIL ${line}`);
    console.log(`${measured} pages measured (${states.length} states x ${SIZES.length} sizes), ${failures.length} failing`);
    return failures.length ? 1 : 0;
  } finally {
    for (const cleanup of cleanups.reverse()) {
      try {
        await cleanup();
      } catch (error) {
        console.error(`scroll-height: cleanup failed: ${error}`);
      }
    }
  }
}

function killTree(child) {
  if (child.exitCode !== null || child.pid === undefined) return;
  if (process.platform === 'win32') spawnSync('taskkill', ['/PID', String(child.pid), '/T', '/F'], { stdio: 'ignore' });
  else child.kill('SIGKILL');
}

async function removeProfile(path) {
  // Edge's children let go of the profile a moment after the kill.
  for (let attempt = 0; attempt < 20; attempt += 1) {
    try {
      rmSync(path, { recursive: true, force: true });
      return;
    } catch {
      await sleep(250);
    }
  }
  rmSync(path, { recursive: true, force: true });
}

main().then(
  (code) => process.exit(code),
  (error) => {
    console.error(`scroll-height: ${error instanceof StartFailure ? 'could not start' : 'failed'}: ${error.message ?? error}`);
    process.exit(2);
  },
);
