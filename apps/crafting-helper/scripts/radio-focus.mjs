// `node scripts/radio-focus.mjs`: proves that, under the Exile theme with the
// delivered radio art (theme/materials/requested/radio-*.svg), keyboard focus
// on a Match radio of the trade panel keeps that radio's visible selection:
// the focus image is a ring with a clear centre, layered over the radio's off
// or on base, never a replacement for it (docs/crafting-helper/design/
// review-steel-masonry.md, "Delivered art").
//
// It starts this app's Vite dev server on a free OS-assigned port (never
// 1422, which `tauri dev` owns), drives the installed Microsoft Edge headless
// through `playwright-core` as `corner-safety.mjs` does, opens
// `/?state=trade-picking&theme=exile` at 960 x 680, device scale 1, and reads
// the Match group's two radios pixel by pixel:
//
//   - art loaded: the document carries `data-art-ch-radio`, else the run
//     would be measuring the CSS placeholder, not the delivered files;
//   - control: unfocused, the checked and the unchecked radio's centres
//     differ, else the instrument cannot see a selection at all;
//   - case checked: Tab until the checked radio has keyboard focus
//     (`:focus-visible` matching); its centre matches its unfocused render,
//     its ring area differs from it, and neither radio's `checked` changed;
//   - case unchecked: a key press, then `focus()` on the unchecked radio
//     (an arrow key would select it); `:focus-visible` matching, its centre
//     matches its unfocused render, its ring area differs, and neither
//     radio's `checked` changed.
//
// Output: one line for the control and one per case, `ok` or `FAIL
// <reasons>`, then `radio-focus: 2 cases, <f> failures, control detected,
// art loaded` (`control not detected`, `art not loaded` when they are not).
// Exit 0 only when nothing failed, the control was detected and the art was
// loaded; 1 on any failure; 2 when the browser or the page cannot start.

import net from 'node:net';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import zlib from 'node:zlib';
import { chromium } from 'playwright-core';
import { decodePng } from './corner-safety-core.mjs';

const APP = path.resolve(fileURLToPath(new URL('..', import.meta.url)));
/** The port `tauri dev` owns (`devUrl`); the check never takes it. */
const DEV_PORT = 1422;
const URL_PATH = '/?state=trade-picking&theme=exile';
const SIZE = { width: 960, height: 680 };
/** The radio is 16 px; the centre is the selected dot's middle, the ring the outer band. */
const RADIO = 16;
const CENTRE = { from: 6, to: 10 };
const RING_FROM = 6.75;

/** The browser or the page did not start: exit 2, not a failure of the rule. */
class StartError extends Error {}

const inflate = (data) => new Uint8Array(zlib.inflateSync(data));

async function freePort() {
  for (;;) {
    const port = await new Promise((resolve, reject) => {
      const server = net.createServer();
      server.once('error', reject);
      server.listen(0, '127.0.0.1', () => {
        const { port } = server.address();
        server.close(() => resolve(port));
      });
    });
    if (port !== DEV_PORT) return port;
  }
}

async function serve() {
  const vite = await import('vite');
  const port = await freePort();
  const server = await vite.createServer({
    root: APP,
    configFile: path.join(APP, 'vite.config.ts'),
    logLevel: 'warn',
    clearScreen: false,
    server: { port, strictPort: true, host: '127.0.0.1', hmr: false },
  });
  await server.listen();
  return { base: `http://127.0.0.1:${port}`, close: () => server.close() };
}

// ----------------------------------------------------------- in-page probes

/** Caret, transitions and animations off; the fonts and two frames settled. */
async function stillInPage() {
  const style = document.createElement('style');
  style.textContent = '*, *::before, *::after { caret-color: transparent !important; animation: none !important; transition: none !important; }';
  document.head.append(style);
  await document.fonts.ready;
  await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
}

async function settleInPage() {
  await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
}

/** The Match radios: which is which, their boxes, and what is focused. */
function radiosInPage() {
  const radios = [...document.querySelectorAll('fieldset.match input[type="radio"]')];
  return {
    art: document.documentElement.hasAttribute('data-art-ch-radio'),
    radios: radios.map((r) => {
      const b = r.getBoundingClientRect();
      return {
        value: r.value,
        checked: r.checked,
        focused: document.activeElement === r,
        focusVisible: r.matches(':focus-visible'),
        rect: { x: b.left, y: b.top, width: b.width, height: b.height },
      };
    }),
  };
}

// ------------------------------------------------------------- node side

/** The radio as rendered: RADIO x RADIO pixels at CSS scale 1. */
async function radioPixels(page, rect) {
  const clip = { x: Math.round(rect.x), y: Math.round(rect.y), width: RADIO, height: RADIO };
  return decodePng(new Uint8Array(await page.screenshot({ type: 'png', clip })), inflate);
}

/** How many pixels of `a` and `b` differ inside the region `keep(x, y)` selects. */
function differing(a, b, keep) {
  let n = 0;
  for (let y = 0; y < a.height; y++) {
    for (let x = 0; x < a.width; x++) {
      if (!keep(x, y)) continue;
      const i = (y * a.width + x) * 4;
      if (a.rgba[i] !== b.rgba[i] || a.rgba[i + 1] !== b.rgba[i + 1] || a.rgba[i + 2] !== b.rgba[i + 2]) n++;
    }
  }
  return n;
}

const inCentre = (x, y) => x >= CENTRE.from && x < CENTRE.to && y >= CENTRE.from && y < CENTRE.to;
const inRing = (x, y) => Math.hypot(x + 0.5 - RADIO / 2, y + 0.5 - RADIO / 2) >= RING_FROM;

async function state(page) {
  await page.evaluate(settleInPage);
  return page.evaluate(radiosInPage);
}

/** One case: the radio focused, compared with its own unfocused render. */
async function check(page, name, value, before, unfocused) {
  const now = await state(page);
  const radio = now.radios.find((r) => r.value === value);
  const reasons = [];
  if (!radio.focused) reasons.push('not focused');
  else if (!radio.focusVisible) reasons.push(':focus-visible does not match');
  const changed = now.radios.filter((r) => r.checked !== before.radios.find((b) => b.value === r.value).checked);
  if (changed.length) reasons.push(`checked changed on ${changed.map((r) => r.value).join(', ')}`);
  const focused = await radioPixels(page, radio.rect);
  const centre = differing(focused, unfocused, inCentre);
  const ring = differing(focused, unfocused, inRing);
  if (centre) reasons.push(`centre differs from its unfocused render (${centre}px): the selection is not kept`);
  if (!ring) reasons.push('ring area unchanged: no focus ring drawn');
  console.log(`radio-focus ${name} (${value}): ${reasons.length ? `FAIL ${reasons.join('; ')}` : `ok (ring ${ring}px, centre kept)`}`);
  return reasons.length ? 1 : 0;
}

async function main() {
  let browser;
  let server;
  try {
    try {
      browser = await chromium.launch({ channel: 'msedge', headless: true });
      server = await serve();
    } catch (error) {
      throw new StartError(`could not start the browser or the dev server: ${String(error?.message || error).split('\n')[0]}`);
    }
    const context = await browser.newContext({ deviceScaleFactor: 1, viewport: SIZE });
    const page = await context.newPage();
    try {
      await page.goto(server.base + URL_PATH, { waitUntil: 'load' });
      await page.waitForSelector('fieldset.match input[type="radio"]', { timeout: 15000 });
    } catch (error) {
      throw new StartError(`could not open ${URL_PATH}: ${String(error?.message || error).split('\n')[0]}`);
    }
    await page.evaluate(stillInPage);
    await page.locator('fieldset.match').scrollIntoViewIfNeeded();
    await page.mouse.move(0, 0);
    await page.evaluate(() => document.activeElement?.blur?.());

    const before = await state(page);
    const checked = before.radios.find((r) => r.checked);
    const unchecked = before.radios.find((r) => !r.checked);
    if (before.radios.length !== 2 || !checked || !unchecked) {
      throw new StartError(`expected two Match radios, one checked; found ${JSON.stringify(before.radios.map((r) => [r.value, r.checked]))}`);
    }
    const art = before.art;
    console.log(`radio-focus art: ${art ? 'ok (data-art-ch-radio set)' : 'FAIL data-art-ch-radio not set: the placeholder would be measured'}`);

    const plain = {
      [checked.value]: await radioPixels(page, checked.rect),
      [unchecked.value]: await radioPixels(page, unchecked.rect),
    };
    const selectionSeen = differing(plain[checked.value], plain[unchecked.value], inCentre);
    const control = selectionSeen > 0;
    console.log(`radio-focus control: ${control ? `ok (unfocused centres differ by ${selectionSeen}px)` : 'FAIL the checked and unchecked centres render the same'}`);

    let failures = 0;
    // Checked: Tab into the group, which lands on its checked radio.
    for (let i = 0; i < 200; i++) {
      await page.keyboard.press('Tab');
      if ((await page.evaluate(radiosInPage)).radios.find((r) => r.value === checked.value).focused) break;
    }
    failures += await check(page, 'checked', checked.value, before, plain[checked.value]);

    // Unchecked: a key press marks the interaction as keyboard; focus() then
    // moves focus without selecting, as an arrow key would.
    await page.evaluate(() => document.activeElement?.blur?.());
    await page.keyboard.press('Shift');
    await page.evaluate((value) => document.querySelector(`fieldset.match input[type="radio"][value="${value}"]`).focus(), unchecked.value);
    failures += await check(page, 'unchecked', unchecked.value, before, plain[unchecked.value]);

    await context.close();
    console.log(`radio-focus: 2 cases, ${failures} failures, ${control ? 'control detected' : 'control not detected'}, ${art ? 'art loaded' : 'art not loaded'}`);
    return failures === 0 && control && art ? 0 : 1;
  } finally {
    if (server) await server.close();
    if (browser) await browser.close();
  }
}

main().then(
  (code) => process.exit(code),
  (error) => {
    console.error(`radio-focus: ${error instanceof StartError ? error.message : error?.stack || error}`);
    process.exit(2);
  },
);
