// `npm run corner-safety`: proves that nothing the Exile theme paints as
// content (text, marks, icons, controls, dividers, scrollbars, focus rings)
// lands on the frame artwork, in every window of the crafting helper, at
// every size, device scale, text size and scroll position of the browser
// previews, by measuring rendered pixels rather than comparing rectangles.
// Adapted from the quest tracker's runner (apps/quest-tracker/scripts/
// corner-safety.mjs); the in-page probes are its own.
//
// It starts this app's Vite dev server on a free OS-assigned port (never
// 1422, which `tauri dev` owns), drives the installed Microsoft Edge
// headless through `playwright-core` (`channel: 'msedge'`, no browser
// download, scrollbars not hidden), and serves the previews with
// `theme=exile`. For each case:
//
//   - renders the artwork alone (every element but `.material-art` hidden,
//     transparent page): alpha >= 64 is art;
//   - collects the client rects of every non-empty text node and the boxes
//     of svg, img, form controls, buttons, scrollbars and elements with a
//     visible border or background, each clipped by every clipping ancestor
//     and the viewport;
//   - fails when any clipped box covers an art pixel, on sideways overflow,
//     and when a box is never fully visible at any scroll position (a
//     button whose text wraps is held line by line, as the quest tracker's).
//
// Matrix (Exile). Main window: `empty`, `grabbed-item`, `craft-planning`,
// `craft-guiding`, `craft-refused`, `craft-done`, `trade-picking`,
// `trade-refused` at 960 x 680 and 600 x 480. Settings: `idle`,
// `available`, `install-downloading`, `install-failed`, `overlay-settings`,
// `trade-league` at 440 x 720. Overlay: `overlay-active` over
// `scene=bright` and `scene=dark` at 480 x 168. Scales are device scale
// factors (these windows have no in-app zoom): 0.75, 1, 1.33 and 2 at text
// 100 %, plus text 200 % at scale 1 (every element's computed font size and
// numeric line height doubled). Each case at the top, middle and end of
// every scroller, after the fonts and two animation frames. Screenshots are
// taken at CSS scale, so a mask pixel is a CSS pixel at every scale.
//
// Checks per case: the overlap, overflow and visibility checks above at
// every position; Tab through every focusable of the main window
// (`craft-planning`) and the Settings window (`idle`) at the default size
// and scale 1, each focused element's 2 px ring fully visible and art-free;
// flush on every main and Settings case (the art within 2 px of each
// viewport edge, and no art pixel cut off by the window, computed from
// `.material-art`'s box, outset and width and the kit PNG's own padding),
// and on the overlay only when it carries a `.material-art`; readability on
// every case: each text run at least 12 px, the caps face (Cinzel) only at
// 13 px and up, a weight of at least 400, no `font-variant-caps`, tracking
// of at most 0.05em below 13 px (0.1em above), and at least 4.5:1 against
// the worst pixel behind it (a disabled control's text exempt; the overlay
// over both scenes).
//
// The control: at every scale a probe over the rendered bottom-left knot of
// the main window's frame must be reported as overlapping, or the run
// fails. The overlay has no frame; the main window's control covers its
// scale.
//
// Flags:
//   --root <app dir>        serve another tree's app
//   --only <substring>      run only the cases whose name contains it
//   --self-check            app-free synthetic pages only: a knot probe that
//                           must overlap and a centre box that must not; a
//                           12 px-inset frame that must be reported not flush
//                           and a flush one that must pass; a 9 px run, a
//                           low-contrast run and a small-caps run that must
//                           be reported and a compliant one that must pass
//   --screenshots <dir>     also write the review images there (scale 1)
//   --references <dir>      with --root: an element screenshot of today's
//                           element for each requested-art id
//   --legacy-base <ref>     compare Claude with <ref> pixel for pixel instead
//   --full                  also text 200 % at every scale (need not pass)
//
// The base worktree (`--legacy-base`, and `--references` given a ref with
// `--root ref:<ref>`): the runner makes a scratch worktree under the OS
// temp folder (`git worktree add --detach <dir> <ref>`), runs `npm ci
// --prefer-offline` in its `apps/crafting-helper`, serves it, and always
// removes it (`git worktree remove --force`, then `git worktree prune`),
// even on failure.
//
// Output: one line per case and position, `ok` or `FAIL <reasons>`, then
// `corner-safety: <n> cases, <f> failures, control detected, flush checked
// on <k> cases, readability checked on <r> cases`. Exit 0 only when nothing
// failed, every declared case ran and the control was detected. With
// `--legacy-base`: one line per case, `identical` or `DIFFER <pixels>`, a
// control line (the first case under Exile must differ from the base, or
// the comparison sees nothing), then `legacy-pixels: <n> cases, <d>
// differ`; exit 0 only when none differ, every declared case ran and the
// control differed.

import { spawn, spawnSync } from 'node:child_process';
import fs from 'node:fs';
import net from 'node:net';
import os from 'node:os';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';
import zlib from 'node:zlib';
import { chromium } from 'playwright-core';
import {
  alphaMask, artPadding, artPixels, clipRect, contrastReasons, decodePng, findOverlaps, flushReasons, grow, legacyLine,
  parseColour, pixelDiff, summaryLine, typeReasons, worstContrast,
} from './corner-safety-core.mjs';

const APP = path.resolve(fileURLToPath(new URL('..', import.meta.url)));
const REPO = path.resolve(APP, '../..');
const KIT_FRAME = path.resolve(APP, 'theme/materials/raster/frame-neutral-steel.png');
/** The port `tauri dev` owns (`devUrl`); the runner never takes it. */
const DEV_PORT = 1422;

const SCALES = ['0.75', '1', '1.33', '2'];
const MAIN_SIZES = [[960, 680], [600, 480]];
const MAIN_STATES = ['empty', 'grabbed-item', 'craft-planning', 'craft-guiding', 'craft-refused', 'craft-done', 'trade-picking', 'trade-refused'];
const SETTINGS_SIZE = [440, 720];
const SETTINGS_STATES = ['idle', 'available', 'install-downloading', 'install-failed', 'overlay-settings', 'trade-league'];
const OVERLAY_SIZE = [480, 168];
const OVERLAY_SCENES = ['bright', 'dark'];
/** Where Tab is walked: the main window's and the Settings window's default case at scale 1. */
const FOCUS_CASES = ['main craft-planning 960x680 s1 t100', 'settings idle 440x720 s1 t100'];
/**
 * The probe on the bottom-left knot, as fractions of the frame's drawn
 * bottom-left corner block (border-image width, from its outset edge): the
 * kit PNG's corner slice is solid artwork there (the quest tracker's).
 */
const KNOT_PROBE = { x: 12 / 48, y: 16 / 48, width: 10 / 48, height: 12 / 48 };
/** How many offenders one line names before it counts the rest. */
const NAMED = 4;
/** Where each page is ready, and the root its probes read. */
const READY = {
  main: '.app[data-state] .material-art',
  settings: '.window[data-state]:not([data-state="none"]) .material-art',
  overlay: '.overlay .panel',
};
/** The same, under Claude (no frame): the legacy comparison and the references. */
const READY_PLAIN = {
  main: '.app[data-state]',
  settings: '.window[data-state]:not([data-state="none"])',
  overlay: '.overlay .panel',
};

/** The legacy comparison's cases: kind, query (without theme) and size. */
const LEGACY_CASES = [
  ...MAIN_STATES.map((s) => ['main', `/?state=${s}`, 960, 680]),
  ['main', '/?state=trade-picking', 600, 480],
  ['main', '/?state=craft-guiding', 600, 480],
  ...SETTINGS_STATES.map((s) => ['settings', `/settings.html?state=${s}`, ...SETTINGS_SIZE]),
  ...OVERLAY_SCENES.map((s) => ['overlay', `/overlay.html?state=overlay-active&scene=${s}`, ...OVERLAY_SIZE]),
];

/** The review images `--screenshots` writes, at scale 1. */
const SCREENSHOTS = [
  ['exile-main-planning', 'main', '/?state=craft-planning&theme=exile', 960, 680],
  ['exile-main-guiding', 'main', '/?state=craft-guiding&theme=exile', 960, 680],
  ['exile-main-min', 'main', '/?state=trade-picking&theme=exile', 600, 480],
  ['exile-settings-idle', 'settings', '/settings.html?state=idle&theme=exile', 440, 720],
  ['exile-overlay-bright', 'overlay', '/overlay.html?state=overlay-active&scene=bright&theme=exile', 480, 168],
  ['exile-overlay-dark', 'overlay', '/overlay.html?state=overlay-active&scene=dark&theme=exile', 480, 168],
  ['claude-main-planning', 'main', '/?state=craft-planning&theme=claude', 960, 680],
  ['claude-settings-idle', 'settings', '/settings.html?state=idle&theme=claude', 440, 720],
  ['claude-overlay-bright', 'overlay', '/overlay.html?state=overlay-active&scene=bright&theme=claude', 480, 168],
];

/**
 * Today's element for each requested-art id (`--references`): the page, its
 * size, the element and, for the update mark no fixture shows, the view
 * field the runner sets in the fixture as it is served.
 */
const REFERENCES = [
  { id: 'ch-radio', kind: 'main', url: '/?state=trade-picking', width: 960, height: 680, selector: 'fieldset.match' },
  { id: 'ch-overlay-frame', kind: 'overlay', url: '/overlay.html?state=overlay-active&scene=dark', width: 480, height: 168, selector: '.overlay .panel' },
  { id: 'ch-item-card', kind: 'main', url: '/?state=craft-planning', width: 960, height: 680, selector: '.item-panel' },
  { id: 'ch-step-marker', kind: 'main', url: '/?state=craft-planning', width: 960, height: 680, selector: 'ol.steps > li:first-child .number' },
  { id: 'ch-update-dot', kind: 'main', url: '/?state=empty', width: 960, height: 680, selector: '.update-mark', update: '0.4.0' },
];

// ---------------------------------------------------------------- arguments

/** A usage mistake: reported in one line, exit 2. */
class UsageError extends Error {}

function parseArgs(argv) {
  // `npm --prefix <app> run` starts the script in the app folder; a relative
  // path means the folder the command was typed in (`INIT_CWD`).
  const from = process.env.INIT_CWD || process.cwd();
  const args = { root: APP, rootRef: null, only: null, screenshots: null, references: null, legacyBase: null, selfCheck: false, full: false };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    const value = () => {
      if (i + 1 >= argv.length) throw new UsageError(`${a} needs a value`);
      return argv[++i];
    };
    if (a === '--root') {
      const v = value();
      if (v.startsWith('ref:')) args.rootRef = v.slice(4);
      else args.root = path.resolve(from, v);
    } else if (a === '--only') args.only = value();
    else if (a === '--screenshots') args.screenshots = path.resolve(from, value());
    else if (a === '--references') args.references = path.resolve(from, value());
    else if (a === '--legacy-base') args.legacyBase = value();
    else if (a === '--self-check') args.selfCheck = true;
    else if (a === '--full') args.full = true;
    else {
      throw new UsageError(
        `unknown argument ${a} (flags: --root <app dir | ref:<git ref>>, --only <substring>, --self-check, --screenshots <dir>, --references <dir>, --legacy-base <ref>, --full)`,
      );
    }
  }
  if (args.references && args.root === APP && !args.rootRef) throw new UsageError('--references needs --root <base app dir> or --root ref:<git ref>');
  if (!args.selfCheck && !args.rootRef && !fs.existsSync(path.join(args.root, 'vite.config.ts'))) {
    throw new UsageError(`--root ${args.root} has no vite.config.ts`);
  }
  return args;
}

// -------------------------------------------------------------------- cases

/** Every declared case, in run order (grouped by scale: one browser context each). */
function declareCases(full) {
  const cases = [];
  const textSets = (scale) => (scale === '1' || full ? [100, 200] : [100]);
  for (const scale of SCALES) {
    for (const text of textSets(scale)) {
      for (const [w, h] of MAIN_SIZES) {
        for (const state of MAIN_STATES) {
          cases.push({ name: `main ${state} ${w}x${h} s${scale} t${text}`, kind: 'main', width: w, height: h, scale, text, url: `/?state=${state}&theme=exile` });
        }
      }
      const [sw, sh] = SETTINGS_SIZE;
      for (const state of SETTINGS_STATES) {
        cases.push({ name: `settings ${state} ${sw}x${sh} s${scale} t${text}`, kind: 'settings', width: sw, height: sh, scale, text, url: `/settings.html?state=${state}&theme=exile` });
      }
      const [ow, oh] = OVERLAY_SIZE;
      for (const scene of OVERLAY_SCENES) {
        cases.push({
          name: `overlay ${scene} ${ow}x${oh} s${scale} t${text}`, kind: 'overlay', width: ow, height: oh, scale, text,
          url: `/overlay.html?state=overlay-active&scene=${scene}&theme=exile`,
        });
      }
    }
  }
  return cases;
}

// ----------------------------------------------------------- in-page probes
// These run inside the page (`page.evaluate` serialises them), so each is
// self-contained.

/** Waits for the fonts and two animation frames. */
async function settleInPage() {
  await document.fonts.ready;
  await new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)));
}

/** Text 200%: every element's computed font size and px line height doubled. */
function doubleTextInPage() {
  const plan = [document.documentElement, ...document.querySelectorAll('*')].map((el) => {
    const cs = getComputedStyle(el);
    return [el, parseFloat(cs.fontSize), cs.lineHeight.endsWith('px') ? parseFloat(cs.lineHeight) : null];
  });
  for (const [el, size, line] of plan) {
    if (!('style' in el)) continue;
    if (size > 0) el.style.setProperty('font-size', `${size * 2}px`, 'important');
    if (line) el.style.setProperty('line-height', `${line * 2}px`, 'important');
  }
}

/** Shows the frame art alone, on a transparent page, or undoes that. */
function maskStyleInPage(on) {
  const id = 'corner-safety-mask';
  document.getElementById(id)?.remove();
  if (!on) return;
  const style = document.createElement('style');
  style.id = id;
  style.textContent = [
    'html, body { background: transparent !important; }',
    '*, *::before, *::after { visibility: hidden !important; }',
    ':not(.material-art) { filter: none !important; }',
    '.material-art, .material-art *, .material-art::before, .material-art::after, .material-art *::before, .material-art *::after { visibility: visible !important; }',
  ].join('\n');
  document.head.append(style);
}

/**
 * Tags the window's scrollers (`data-cs-scroller`) and scrolls them:
 * `positions` is null to list them, or `{ index, where }` to scroll one
 * (`top`, `mid`, `end`) with every other at the top.
 */
function scrollInPage(target) {
  const root = document.querySelector('.app[data-state]') || document.querySelector('.window') || document.querySelector('.overlay');
  if (!root) return [];
  const scrollers = [root, ...root.querySelectorAll('*')].filter((el) => {
    const cs = getComputedStyle(el);
    return /(auto|scroll)/.test(cs.overflowY) && el.scrollHeight > el.clientHeight + 1;
  });
  const out = scrollers.map((el, i) => {
    el.setAttribute('data-cs-scroller', String(i));
    const classes = [...el.classList].slice(0, 2).map((c) => `.${c}`).join('');
    return { index: i, label: `${el.tagName.toLowerCase()}${classes}` };
  });
  for (const [i, el] of scrollers.entries()) {
    const max = el.scrollHeight - el.clientHeight;
    const where = target && target.index === i ? target.where : 'top';
    el.scrollTop = where === 'end' ? max : where === 'mid' ? Math.round(max / 2) : 0;
  }
  return out;
}

/**
 * Everything one position needs: the content boxes with their clips, and
 * the facts the other checks read. Rects are viewport px (what the
 * screenshot holds at device scale 1); `offsetWidth`, `clientWidth` and
 * computed lengths are unzoomed CSS px, so each is scaled by the element's
 * own ratio of the two.
 */
function measureInPage() {
  const root = document.querySelector('.app[data-state]') || document.querySelector('.window') || document.querySelector('.overlay');
  if (!root) return { error: 'no .app, .window or .overlay rendered' };
  const EXCLUDED = '.material-art, [data-cs-probe], .visually-hidden';
  const WELL_ART = /masonry-charcoal/;
  const CONTROLS = 'img, .status-mark, .icon, input, select, textarea, button, [role="button"], [role="switch"], [role="slider"], [role="checkbox"], [role="radio"], [role="tab"], [role="combobox"], [role="option"]';
  const viewport = { left: 0, top: 0, right: window.innerWidth, bottom: window.innerHeight };
  const plain = (r) => ({ left: r.left, top: r.top, right: r.right, bottom: r.bottom });
  const px = (v) => (typeof v === 'string' && v.endsWith('px') ? parseFloat(v) : NaN);
  const alpha = (colour) => {
    if (!colour || colour === 'transparent') return 0;
    const slash = colour.match(/\/\s*([\d.]+)(%?)\s*\)$/);
    if (slash) return parseFloat(slash[1]) / (slash[2] ? 100 : 1);
    const rgba = colour.match(/^rgba\([^)]*,\s*([\d.]+)\s*\)$/);
    return rgba ? parseFloat(rgba[1]) : 1;
  };
  const describe = (el) => {
    const classes = [...el.classList].slice(0, 2).map((c) => `.${c}`).join('');
    const text = (el.textContent || el.getAttribute('aria-label') || '').trim().replace(/\s+/g, ' ').slice(0, 24);
    return `${el.tagName.toLowerCase()}${classes}${text ? ` "${text}"` : ''}`;
  };
  const ratio = (el) => {
    const r = el.getBoundingClientRect();
    return el.offsetWidth > 0 ? r.width / el.offsetWidth : (el.currentCSSZoom || 1);
  };
  const paddingBox = (el) => {
    const cs = getComputedStyle(el);
    const r = el.getBoundingClientRect();
    const z = ratio(el);
    const left = r.left + el.clientLeft * z;
    const top = r.top + el.clientTop * z;
    return {
      left: cs.overflowX === 'visible' ? -Infinity : left,
      right: cs.overflowX === 'visible' ? Infinity : left + el.clientWidth * z,
      top: cs.overflowY === 'visible' ? -Infinity : top,
      bottom: cs.overflowY === 'visible' ? Infinity : top + el.clientHeight * z,
    };
  };
  const clips = (cs) => cs.overflowX !== 'visible' || cs.overflowY !== 'visible' || /paint|strict|content/.test(cs.contain);
  const clipCache = new Map();
  /** The clip rects over `el`'s paint: its clipping ancestors (and itself, for its own text), then the viewport. */
  const clipsFor = (el, self) => {
    const key = el;
    if (!self && clipCache.has(key)) return clipCache.get(key);
    const out = [];
    for (let a = self ? el : el.parentElement; a && a !== document.body && a !== document.documentElement; a = a.parentElement) {
      if (clips(getComputedStyle(a))) out.push(paddingBox(a));
    }
    out.push(viewport);
    if (!self) clipCache.set(key, out);
    return out;
  };
  const intersect = (a, b) => {
    const r = { left: Math.max(a.left, b.left), top: Math.max(a.top, b.top), right: Math.min(a.right, b.right), bottom: Math.min(a.bottom, b.bottom) };
    return r.right > r.left && r.bottom > r.top ? r : null;
  };
  const scrollerOf = (el) => {
    for (let a = el; a && a !== document.body; a = a.parentElement) {
      if (/(auto|scroll)/.test(getComputedStyle(a).overflowY)) return a;
    }
    return null;
  };
  /**
   * The window a scroller shows its content through. A scroller inside
   * another (the Settings window's currency list inside its sections) that
   * fits the outer one's window is shown whole at some position of the
   * outer one: its window is then its own padding box (`nested`), the
   * crafting helper's addition to the quest tracker's check. Otherwise it is
   * what its clipping ancestors leave of it now.
   */
  const windowOf = (scroller) => {
    const own = paddingBox(scroller);
    let clip = own;
    for (const c of clipsFor(scroller, false)) clip = clip && intersect(clip, c);
    const outer = scroller.parentElement ? scrollerOf(scroller.parentElement) : null;
    if (outer && outer !== scroller && root.contains(outer)) {
      const outerClip = windowOf(outer).clip;
      if (outerClip && own.bottom - own.top <= outerClip.bottom - outerClip.top + 0.5 && own.right - own.left <= outerClip.right - outerClip.left + 0.5) {
        return { clip: { left: Math.max(own.left, outerClip.left), right: Math.min(own.right, outerClip.right), top: own.top, bottom: own.bottom }, nested: true };
      }
    }
    return { clip, nested: false };
  };
  /** Whether an element between `el` and `scroller` cuts its line with an ellipsis. */
  const ellipsized = (el, scroller) => {
    for (let a = el; a && a !== scroller; a = a.parentElement) {
      const cs = getComputedStyle(a);
      if (cs.textOverflow === 'ellipsis' && cs.overflowX !== 'visible' && cs.whiteSpace === 'nowrap') return true;
    }
    return false;
  };
  const visible = (el) => el.checkVisibility({ opacityProperty: true, visibilityProperty: true });
  const excluded = (el) => el === root || !!el.closest(EXCLUDED) || WELL_ART.test(getComputedStyle(el).backgroundImage);
  const hasBorder = (cs) => ['Top', 'Right', 'Bottom', 'Left'].some((s) =>
    parseFloat(cs[`border${s}Width`]) > 0 && !/none|hidden/.test(cs[`border${s}Style`]) && alpha(cs[`border${s}Color`]) > 0);
  const hasBackground = (cs) => alpha(cs.backgroundColor) > 0 || (cs.backgroundImage && cs.backgroundImage !== 'none');

  const boxes = [];
  const problems = [];
  const unreachable = [];
  const cutScrollers = new Set();
  /** A box that holds other content (text, a graphic, a control) rather than being one. */
  const holder = (el) => !!el.textContent?.trim() || !!el.querySelector(`svg, ${CONTROLS}`);
  /**
   * A button holding text that wraps onto two or more lines. When it is taller than its scroller it can never be seen
   * whole; its lines and marks are measured one by one instead, each of
   * which must fit (they are boxes of their own).
   */
  const wrapsText = (el) => {
    if (!el.matches('button, [role="button"]')) return false;
    const tops = new Set();
    const lines = document.createTreeWalker(el, NodeFilter.SHOW_TEXT);
    const r = document.createRange();
    for (let node = lines.nextNode(); node; node = lines.nextNode()) {
      if (!node.data.trim()) continue;
      r.selectNodeContents(node);
      for (const rect of r.getClientRects()) tops.add(Math.round(rect.top));
      if (tops.size > 1) return true;
    }
    return false;
  };
  const addBox = (label, rect, el, self, sized) => {
    if (rect.right - rect.left <= 0 || rect.bottom - rect.top <= 0) return;
    const box = { label, rect: plain(rect), clips: clipsFor(el, self) };
    boxes.push(box);
    // Every line, graphic and control must fit its scroller's clip, or part
    // of it is never visible. A background that holds them may scroll past
    // (its contents are measured themselves); it must still keep off the art.
    if (!sized) return;
    const scroller = scrollerOf(self ? el : el.parentElement);
    if (!scroller || !root.contains(scroller)) {
      // Nothing scrolls it into view, so it must be fully on screen now (a
      // horizontal cut is left alone: an ellipsis is a design choice).
      let shown = box.rect;
      for (const c of box.clips) shown = shown && intersect(shown, c);
      if (!shown || shown.top > rect.top + 0.5 || shown.bottom < rect.bottom - 0.5) unreachable.push(`${label} cut off, and nothing scrolls it`);
    } else {
      const own = paddingBox(scroller);
      const { clip, nested } = windowOf(scroller);
      // A scroller that runs off screen never shows what scrolls to its end
      // (one inside another that can scroll it whole into view is not cut).
      if (clip && !nested && !cutScrollers.has(scroller) && (clip.top > own.top + 0.5 || clip.bottom < own.bottom - 0.5)) {
        cutScrollers.add(scroller);
        unreachable.push(`scroller ${describe(scroller).split(' ')[0]} is cut off (shows ${Math.round(clip.top)}-${Math.round(clip.bottom)} of ${Math.round(own.top)}-${Math.round(own.bottom)})`);
      }
      const w = rect.right - rect.left;
      const h = rect.bottom - rect.top;
      // A line an element below the scroller cuts with an ellipsis (the
      // overlay's one-line rows) is a design choice there too, as the
      // quest tracker's check treats a cut no scroller reaches.
      const wide = w > clip?.right - clip?.left + 0.5 && !(self && ellipsized(el, scroller));
      if (!clip) unreachable.push(`${label} in a hidden scroller`);
      else if ((h > clip.bottom - clip.top + 0.5 && (self || !wrapsText(el))) || wide) {
        unreachable.push(`${label} ${Math.round(w)}x${Math.round(h)} > scroller ${Math.round(clip.right - clip.left)}x${Math.round(clip.bottom - clip.top)}`);
      }
    }
  };

  // Elements: graphics, controls, and anything with a border or background.
  for (const el of root.querySelectorAll('*')) {
    if (excluded(el) || !visible(el)) continue;
    if (el instanceof SVGElement) {
      if (el.tagName.toLowerCase() === 'svg' && !el.parentElement?.closest('svg')) addBox(describe(el), el.getBoundingClientRect(), el, false, true);
      continue;
    }
    const cs = getComputedStyle(el);
    const control = el.matches(CONTROLS);
    let include = control || hasBorder(cs) || hasBackground(cs);
    // A pseudo-element decoration must stay inside its owner's box; the
    // owner's box then stands for it.
    for (const pseudo of ['::before', '::after']) {
      const ps = getComputedStyle(el, pseudo);
      if (ps.content === 'none' || ps.content === 'normal' || ps.display === 'none') continue;
      const painted = ps.content !== '""' || hasBorder(ps) || hasBackground(ps) || ps.boxShadow !== 'none' || (ps.maskImage && ps.maskImage !== 'none');
      if (!painted) continue;
      include = true;
      const name = `${describe(el)}${pseudo}`;
      const margins = ['Top', 'Right', 'Bottom', 'Left'].map((s) => px(ps[`margin${s}`]));
      if (margins.some((m) => m < -0.5)) problems.push(`${name} has a negative margin`);
      if (ps.position === 'fixed') problems.push(`${name} is fixed, outside its owner`);
      else if (ps.position === 'absolute') {
        if (cs.position === 'static') { problems.push(`${name} is placed against an ancestor, not its owner`); continue; }
        const content = ps.boxSizing !== 'border-box';
        const extra = (a, b) => (content ? px(ps[`padding${a}`]) + px(ps[`padding${b}`]) + px(ps[`border${a}Width`]) + px(ps[`border${b}Width`]) : 0);
        let x0 = px(ps.left) + margins[3];
        let y0 = px(ps.top) + margins[0];
        const w = px(ps.width) + extra('Left', 'Right');
        const h = px(ps.height) + extra('Top', 'Bottom');
        let x1 = x0 + w;
        let y1 = y0 + h;
        if ([x0, y0, x1, y1].some(Number.isNaN)) continue;
        // A 2D transform (a rotated diamond) paints its transformed corners.
        const m = ps.transform.match(/^matrix\(([^)]+)\)$/);
        if (m) {
          const [a, b, c, d, e, f] = m[1].split(',').map(Number);
          const [ox, oy] = ps.transformOrigin.split(' ').map(px);
          const corners = [[0, 0], [w, 0], [0, h], [w, h]].map(([x, y]) => [
            x0 + ox + a * (x - ox) + c * (y - oy) + e,
            y0 + oy + b * (x - ox) + d * (y - oy) + f,
          ]);
          x0 = Math.min(...corners.map((p) => p[0])); x1 = Math.max(...corners.map((p) => p[0]));
          y0 = Math.min(...corners.map((p) => p[1])); y1 = Math.max(...corners.map((p) => p[1]));
        }
        // The owner's border box, from its padding edge (the containing block).
        const inside = x0 >= -el.clientLeft - 0.5 && y0 >= -el.clientTop - 0.5 &&
          x1 <= el.offsetWidth - el.clientLeft + 0.5 && y1 <= el.offsetHeight - el.clientTop + 0.5;
        if (!inside) problems.push(`${name} reaches outside its owner`);
      } else if (ps.position === 'relative' && (px(ps.left) < -0.5 || px(ps.top) < -0.5)) {
        problems.push(`${name} is shifted outside its owner`);
      }
    }
    if (include) addBox(describe(el), el.getBoundingClientRect(), el, false, control || !holder(el));
  }

  // Scrollbars of the panel's scrollers.
  for (const el of [root, ...root.querySelectorAll('[data-cs-scroller]')]) {
    if (!(el instanceof HTMLElement) || el.closest(EXCLUDED)) continue;
    const cs = getComputedStyle(el);
    if (!/(auto|scroll)/.test(cs.overflowY) || el.scrollHeight <= el.clientHeight + 1) continue;
    const bar = el.offsetWidth - el.clientWidth - el.clientLeft - (px(cs.borderRightWidth) || 0);
    if (bar <= 0.5) continue;
    const r = el.getBoundingClientRect();
    const z = ratio(el);
    const left = r.left + (el.clientLeft + el.clientWidth) * z;
    const top = r.top + el.clientTop * z;
    boxes.push({ label: `scrollbar of ${describe(el).split(' ')[0]}`, rect: { left, top, right: left + bar * z, bottom: top + el.clientHeight * z }, clips: clipsFor(el, false) });
  }

  // Text, one rect per line fragment.
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  const range = document.createRange();
  for (let node = walker.nextNode(); node; node = walker.nextNode()) {
    const text = node.data.trim();
    const parent = node.parentElement;
    if (!text || !parent || parent.closest(EXCLUDED) || !visible(parent)) continue;
    range.selectNodeContents(node);
    for (const r of range.getClientRects()) addBox(`text "${text.replace(/\s+/g, ' ').slice(0, 24)}"`, r, parent, true, true);
  }

  // Sideways overflow, of the page and of every scroller in the panel.
  const sideways = [];
  const page = document.scrollingElement || document.documentElement;
  if (page.scrollWidth > page.clientWidth + 1) sideways.push(`page ${page.scrollWidth}>${page.clientWidth}`);
  for (const el of [root, ...root.querySelectorAll('*')]) {
    if (!(el instanceof HTMLElement) || el.closest(EXCLUDED)) continue;
    if (/(auto|scroll)/.test(getComputedStyle(el).overflowX) && el.scrollWidth > el.clientWidth + 1) {
      sideways.push(`${describe(el).split(' ')[0]} ${el.scrollWidth}>${el.clientWidth}`);
    }
  }

  const art = document.querySelector('.material-art');
  return {
    boxes,
    problems,
    unreachable,
    sideways,
    art: art ? plain(art.getBoundingClientRect()) : null,
  };
}

/**
 * Every text run the window shows at this scroll position, for the
 * readability check: its computed type, its colour with the opacity of
 * everything above it, and its line rects clipped to what is on screen. A
 * run clipped away here is left for the position that shows it.
 */
function textRunsInPage() {
  const root = document.querySelector('.app[data-state]') || document.querySelector('.window') || document.querySelector('.overlay');
  if (!root) return [];
  const EXCLUDED = '.material-art, [data-cs-probe], .visually-hidden';
  const viewport = { left: 0, top: 0, right: window.innerWidth, bottom: window.innerHeight };
  const ratio = (el) => {
    const r = el.getBoundingClientRect();
    return el.offsetWidth > 0 ? r.width / el.offsetWidth : (el.currentCSSZoom || 1);
  };
  const clipOf = (a) => {
    const cs = getComputedStyle(a);
    if (cs.overflowX === 'visible' && cs.overflowY === 'visible' && !/paint|strict|content/.test(cs.contain)) return null;
    const r = a.getBoundingClientRect();
    const z = ratio(a);
    const left = r.left + a.clientLeft * z;
    const top = r.top + a.clientTop * z;
    return {
      left: cs.overflowX === 'visible' ? -Infinity : left,
      right: cs.overflowX === 'visible' ? Infinity : left + a.clientWidth * z,
      top: cs.overflowY === 'visible' ? -Infinity : top,
      bottom: cs.overflowY === 'visible' ? Infinity : top + a.clientHeight * z,
    };
  };
  const intersect = (a, b) => {
    const r = { left: Math.max(a.left, b.left), top: Math.max(a.top, b.top), right: Math.min(a.right, b.right), bottom: Math.min(a.bottom, b.bottom) };
    return r.right > r.left && r.bottom > r.top ? r : null;
  };
  const runs = [];
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  const range = document.createRange();
  for (let node = walker.nextNode(); node; node = walker.nextNode()) {
    const text = node.data.trim();
    const parent = node.parentElement;
    if (!text || !parent || parent.closest(EXCLUDED)) continue;
    if (!parent.checkVisibility({ opacityProperty: true, visibilityProperty: true })) continue;
    const clips = [];
    for (let a = parent; a && a !== document.body && a !== document.documentElement; a = a.parentElement) {
      const c = clipOf(a);
      if (c) clips.push(c);
    }
    clips.push(viewport);
    range.selectNodeContents(node);
    const rects = [];
    for (const r of range.getClientRects()) {
      let shown = { left: r.left, top: r.top, right: r.right, bottom: r.bottom };
      for (const c of clips) shown = shown && intersect(shown, c);
      if (shown) rects.push(shown);
    }
    if (!rects.length) continue;
    const cs = getComputedStyle(parent);
    let opacity = 1;
    for (let a = parent; a; a = a.parentElement) opacity *= parseFloat(getComputedStyle(a).opacity);
    runs.push({
      label: `text "${text.replace(/\s+/g, ' ').slice(0, 24)}"`,
      fontSize: parseFloat(cs.fontSize),
      zoom: parent.currentCSSZoom || ratio(parent),
      variantCaps: cs.fontVariantCaps,
      weight: parseFloat(cs.fontWeight),
      letterSpacing: cs.letterSpacing === 'normal' ? 0 : parseFloat(cs.letterSpacing),
      capsFace: /cinzel/i.test(cs.fontFamily.split(',')[0]),
      colour: cs.color,
      opacity,
      disabled: !!parent.closest(':disabled, [aria-disabled="true"]'),
      rects,
    });
  }
  return runs;
}

/** Makes every text run transparent (no shadow either), or undoes that: the contrast check's background. */
function hideTextInPage(on) {
  const id = 'corner-safety-no-text';
  document.getElementById(id)?.remove();
  if (!on) return;
  const style = document.createElement('style');
  style.id = id;
  style.textContent = '*, *::before, *::after, *::placeholder { color: transparent !important; -webkit-text-fill-color: transparent !important; text-shadow: none !important; text-decoration-color: transparent !important; caret-color: transparent !important; }';
  document.head.append(style);
}

/** Places the knot probe and returns its box and clips, or removes it. */
function probeInPage(options) {
  document.querySelectorAll('[data-cs-probe]').forEach((el) => el.remove());
  if (!options) return null;
  const art = document.querySelector('.material-art');
  const root = document.querySelector('.app[data-state]') || document.querySelector('.window') || document.querySelector('.overlay');
  if (!art || !root) return null;
  // Beside the window's content, not in it, so no clip of its own can hide it:
  // the control asks whether art under a painted box is seen, nothing else.
  const host = art.parentElement;
  const probe = document.createElement('div');
  probe.setAttribute('data-cs-probe', '');
  probe.style.cssText = 'position:absolute;left:0;top:0;width:10px;height:10px;background:#f0f;z-index:99;pointer-events:none';
  host.append(probe);
  // The bottom-left corner block as the frame really draws it: its
  // border-image width and outset (computed, unzoomed px) in viewport px.
  const a = art.getBoundingClientRect();
  const cs = getComputedStyle(art);
  const zoom = art.offsetWidth > 0 ? a.width / art.offsetWidth : 1;
  const length = (v, fallback) => (v.endsWith('px') ? parseFloat(v) : fallback) * zoom;
  const corner = length(cs.borderImageWidth.split(' ')[0], 48);
  const outset = length(cs.borderImageOutset.split(' ')[0], 0);
  const block = { left: a.left - outset, bottom: a.bottom + outset };
  const want = {
    left: block.left + corner * options.x,
    top: block.bottom - corner + corner * options.y,
    width: corner * options.width,
    height: corner * options.height,
  };
  // Correct the placement by what the browser actually did (zoom, filters
  // and transforms all move an absolute box).
  for (let i = 0; i < 2; i++) {
    const r = probe.getBoundingClientRect();
    const z = probe.offsetWidth > 0 ? r.width / probe.offsetWidth : 1;
    probe.style.left = `${parseFloat(probe.style.left) + (want.left - r.left) / z}px`;
    probe.style.top = `${parseFloat(probe.style.top) + (want.top - r.top) / z}px`;
    probe.style.width = `${want.width / z}px`;
    probe.style.height = `${want.height / z}px`;
  }
  const r = probe.getBoundingClientRect();
  const clips = [];
  for (let el = probe.parentElement; el && el !== document.body && el !== document.documentElement; el = el.parentElement) {
    const cs = getComputedStyle(el);
    if (cs.overflowX !== 'visible' || cs.overflowY !== 'visible') {
      const b = el.getBoundingClientRect();
      const z = el.offsetWidth > 0 ? b.width / el.offsetWidth : 1;
      const left = b.left + el.clientLeft * z;
      const top = b.top + el.clientTop * z;
      clips.push({ left, top, right: left + el.clientWidth * z, bottom: top + el.clientHeight * z });
    }
  }
  clips.push({ left: 0, top: 0, right: window.innerWidth, bottom: window.innerHeight });
  return { label: 'knot probe', rect: { left: r.left, top: r.top, right: r.right, bottom: r.bottom }, clips };
}

/** The focused element's box and clips, tagging it so a cycle is seen. */
function focusedInPage() {
  const el = document.activeElement;
  if (!el || el === document.body || el === document.documentElement) return null;
  if (!el.hasAttribute('data-cs-focus')) el.setAttribute('data-cs-focus', String(document.querySelectorAll('[data-cs-focus]').length));
  const r = el.getBoundingClientRect();
  const zoom = el.offsetWidth > 0 ? r.width / el.offsetWidth : (el.currentCSSZoom || 1);
  const clips = [];
  for (let a = el.parentElement; a && a !== document.body && a !== document.documentElement; a = a.parentElement) {
    const cs = getComputedStyle(a);
    if (cs.overflowX !== 'visible' || cs.overflowY !== 'visible') {
      const b = a.getBoundingClientRect();
      const z = a.offsetWidth > 0 ? b.width / a.offsetWidth : 1;
      const left = b.left + a.clientLeft * z;
      const top = b.top + a.clientTop * z;
      clips.push({
        left: cs.overflowX === 'visible' ? -Infinity : left,
        right: cs.overflowX === 'visible' ? Infinity : left + a.clientWidth * z,
        top: cs.overflowY === 'visible' ? -Infinity : top,
        bottom: cs.overflowY === 'visible' ? Infinity : top + a.clientHeight * z,
      });
    }
  }
  clips.push({ left: 0, top: 0, right: window.innerWidth, bottom: window.innerHeight });
  const classes = [...el.classList].slice(0, 2).map((c) => `.${c}`).join('');
  const label = `${el.tagName.toLowerCase()}${classes} "${(el.getAttribute('aria-label') || el.textContent || '').trim().replace(/\s+/g, ' ').slice(0, 24)}"`;
  const art = document.querySelector('.material-art')?.getBoundingClientRect();
  // A button whose text wraps and whose ring is taller than the window its
  // clips leave can never show that ring whole: only its left and right
  // edges are held to it. Its lines are checked one by one at the scroll
  // positions.
  let wraps = false;
  if (el.matches('button, [role="button"]')) {
    const tops = new Set();
    const lines = document.createTreeWalker(el, NodeFilter.SHOW_TEXT);
    const range = document.createRange();
    for (let node = lines.nextNode(); node && tops.size < 2; node = lines.nextNode()) {
      if (!node.data.trim()) continue;
      range.selectNodeContents(node);
      for (const rect of range.getClientRects()) tops.add(Math.round(rect.top));
    }
    wraps = tops.size > 1;
  }
  const window_ = clips.reduce((w, c) => ({ top: Math.max(w.top, c.top), bottom: Math.min(w.bottom, c.bottom) }), { top: -Infinity, bottom: Infinity });
  const tall = wraps && r.height + 4 * zoom > window_.bottom - window_.top + 0.5;
  return {
    key: el.getAttribute('data-cs-focus'),
    label,
    rect: { left: r.left, top: r.top, right: r.right, bottom: r.bottom },
    clips,
    zoom,
    tall,
    art: art ? { left: art.left, top: art.top, right: art.right, bottom: art.bottom } : null,
  };
}

/**
 * The frame as the page draws it, for the flush check: `.material-art`'s
 * box and its border-image outset, width and slice per side. Computed
 * lengths are unzoomed CSS px, so each is scaled by the element's own zoom
 * into viewport px; a bare number is a multiple of the (zero) border width.
 */
function frameInPage() {
  const art = document.querySelector('.material-art');
  if (!art) return null;
  const r = art.getBoundingClientRect();
  const cs = getComputedStyle(art);
  const zoom = art.offsetWidth > 0 ? r.width / art.offsetWidth : (art.currentCSSZoom || 1);
  const border = { top: cs.borderTopWidth, right: cs.borderRightWidth, bottom: cs.borderBottomWidth, left: cs.borderLeftWidth };
  // A one-to-four value list, as CSS expands it to top, right, bottom, left.
  const sides = (value) => {
    const v = value.replace(/\bfill\b/, '').trim().split(/\s+/);
    const [top, right = top, bottom = top, left = right] = v;
    return { top, right, bottom, left };
  };
  const lengths = (value) => {
    const s = sides(value);
    const out = {};
    for (const k of ['top', 'right', 'bottom', 'left']) {
      const v = s[k];
      out[k] = (v.endsWith('px') ? parseFloat(v) : parseFloat(v) * parseFloat(border[k])) * zoom;
    }
    return out;
  };
  const slice = sides(cs.borderImageSlice);
  return {
    rect: { left: r.left, top: r.top, right: r.right, bottom: r.bottom },
    outset: lengths(cs.borderImageOutset),
    width: lengths(cs.borderImageWidth),
    slice: { top: parseFloat(slice.top), right: parseFloat(slice.right), bottom: parseFloat(slice.bottom), left: parseFloat(slice.left) },
  };
}

// ------------------------------------------------------------- node side

const inflate = (data) => new Uint8Array(zlib.inflateSync(data));

async function settle(page) {
  await page.evaluate(settleInPage);
}

/** A screenshot at CSS scale, decoded: one pixel per CSS pixel at any device scale. */
async function shot(page, options = {}) {
  return decodePng(new Uint8Array(await page.screenshot({ type: 'png', scale: 'css', ...options })), inflate);
}

/** The artwork mask of the page as it stands. */
async function takeMask(page) {
  await page.evaluate(maskStyleInPage, true);
  await settle(page);
  const image = await shot(page, { omitBackground: true });
  await page.evaluate(maskStyleInPage, false);
  await settle(page);
  return alphaMask(image);
}

/**
 * The page as it stands with every text run transparent: what lies behind
 * the text. Not `omitBackground`: where a window lets the page through (the
 * overlay), the preview's scene stands in for whatever is behind it.
 */
async function takeBackground(page) {
  await page.evaluate(hideTextInPage, true);
  await settle(page);
  const image = await shot(page);
  await page.evaluate(hideTextInPage, false);
  await settle(page);
  return image;
}

/**
 * The readability check at one scroll position: each run on screen against
 * the 12 px floor, the caps, weight and tracking rules, and 4.5:1 against
 * the worst pixel behind it (a disabled control's text is exempt from
 * contrast, as WCAG exempts it). Each reason names its run.
 */
async function readabilityReasons(page) {
  const runs = await page.evaluate(textRunsInPage);
  if (!runs.length) return { reasons: [], runs: 0 };
  const image = await takeBackground(page);
  const reasons = [];
  for (const run of runs) {
    const own = typeReasons(run);
    if (!run.disabled) {
      const fg = parseColour(run.colour);
      if (fg) own.push(...contrastReasons(worstContrast({ ...fg, a: fg.a * run.opacity }, image, run.rects)));
      else own.push(`colour ${run.colour} not read`);
    }
    for (const r of own) reasons.push(`${run.label} ${r}`);
  }
  return { reasons, runs: runs.length };
}

const sameRect = (a, b, tolerance = 0.5) =>
  !!a && !!b && ['left', 'top', 'right', 'bottom'].every((k) => Math.abs(a[k] - b[k]) <= tolerance);

/** Why the knot probe was not reported: no frame, off screen, or no art under it. */
function controlMiss(probe) {
  if (!probe) return 'no .material-art to probe';
  const r = probe.rect;
  const at = `probe at ${Math.round(r.left)},${Math.round(r.top)}`;
  return clipRect(r, probe.clips) ? `${at} saw no art` : `${at} is off screen or clipped away`;
}

const named = (list) => `${list.slice(0, NAMED).join(', ')}${list.length > NAMED ? `, +${list.length - NAMED} more` : ''}`;

/** The reasons one position fails, from its measurement and mask. */
function positionReasons(m, mask) {
  const reasons = [];
  const overlaps = findOverlaps(mask, m.boxes);
  if (overlaps.length) reasons.push(`art under ${overlaps.length} boxes: ${named(overlaps.map((o) => `${o.label} (${o.pixels}px)`))}`);
  if (m.sideways.length) reasons.push(`sideways overflow: ${named(m.sideways)}`);
  if (m.unreachable.length) reasons.push(`never fully visible: ${named(m.unreachable)}`);
  if (m.problems.length) reasons.push(named(m.problems));
  return reasons;
}

/** Tab through the window: each focused element, with its 2px ring, fully visible and art-free. */
async function focusReasons(page, topMask, topArt) {
  await page.evaluate(() => {
    document.activeElement?.blur?.();
    document.querySelectorAll('[data-cs-focus]').forEach((el) => el.removeAttribute('data-cs-focus'));
  });
  await page.evaluate(scrollInPage, null);
  const reasons = [];
  const seen = new Set();
  let count = 0;
  for (let i = 0; i < 400; i++) {
    await page.keyboard.press('Tab');
    await settle(page);
    const f = await page.evaluate(focusedInPage);
    if (!f || seen.has(f.key)) break;
    seen.add(f.key);
    count++;
    const ring = grow(f.rect, 2 * f.zoom);
    const shown = clipRect(ring, f.clips);
    if (!shown) reasons.push(`${f.label} hidden when focused`);
    else {
      const sides = f.tall ? ['left', 'right'] : ['left', 'top', 'right', 'bottom'];
      const cut = [['left', shown.left - ring.left], ['top', shown.top - ring.top], ['right', ring.right - shown.right], ['bottom', ring.bottom - shown.bottom]]
        .filter(([side, by]) => sides.includes(side) && by > 0.5).map(([side, by]) => `${side} ${Math.round(by * 10) / 10}px`);
      if (cut.length) reasons.push(`${f.label} clipped when focused (${cut.join(', ')})`);
    }
    const mask = sameRect(f.art, topArt) ? topMask : await takeMask(page);
    const pixels = shown ? artPixels(mask, shown) : 0;
    if (pixels > 0) reasons.push(`${f.label} focused over art (${pixels}px)`);
  }
  return { reasons, count };
}

/** Opens a case's page and waits until it drew its first view. */
async function open(page, base, url, ready, width, height) {
  await page.setViewportSize({ width, height });
  await page.goto(base + url, { waitUntil: 'load' });
  await page.waitForSelector(ready, { timeout: 15000 });
  await settle(page);
}

/**
 * One case: every scroll position, the flush check, focus where declared,
 * and the control when asked. `padding` is the frame PNG's own.
 */
async function runCase(page, base, c, withControl, padding, print) {
  await open(page, base, c.url, READY[c.kind], c.width, c.height);
  if (c.text === 200) {
    await page.evaluate(doubleTextInPage);
    await settle(page);
    await settle(page);
  }
  const scrollers = await page.evaluate(scrollInPage, null);
  const positions = [{ label: 'top', target: null }];
  for (const s of scrollers) {
    const tag = scrollers.length > 1 ? `:${s.label}` : '';
    positions.push({ label: `mid${tag}`, target: { index: s.index, where: 'mid' } });
    positions.push({ label: `end${tag}`, target: { index: s.index, where: 'end' } });
  }
  let failures = 0;
  let control = null;
  let topMask = null;
  let topArt = null;
  let hasArt = false;
  const unreadable = new Set();
  let runs = 0;
  for (const p of positions) {
    await page.evaluate(scrollInPage, p.target);
    await settle(page);
    const mask = await takeMask(page);
    const m = await page.evaluate(measureInPage);
    if (m.error) throw new Error(m.error);
    if (!topMask) {
      topMask = mask;
      topArt = m.art;
      hasArt = !!m.art;
    }
    const readable = await readabilityReasons(page);
    runs += readable.runs;
    for (const r of readable.reasons) unreadable.add(r);
    if (withControl && control === null) {
      const probe = await page.evaluate(probeInPage, KNOT_PROBE);
      await page.evaluate(probeInPage, null);
      control = !!probe && findOverlaps(mask, [probe]).length === 1;
      print(`${c.name} @control: ${control ? 'ok' : `FAIL control not detected (${controlMiss(probe)})`}`);
      if (!control) failures++;
    }
    const reasons = positionReasons(m, mask);
    print(`${c.name} @${p.label}: ${reasons.length ? `FAIL ${reasons.join('; ')}` : 'ok'}`);
    if (reasons.length) failures++;
  }
  print(`${c.name} @readability: ${unreadable.size ? `FAIL ${named([...unreadable])}` : `ok (${runs} runs)`}`);
  if (unreadable.size) failures++;
  // Flush: the frame is fixed to the window, so the top position's mask stands for all.
  let flushed = 0;
  if (c.kind !== 'overlay' || hasArt) {
    const reasons = flushReasons(topMask, await page.evaluate(frameInPage), padding);
    print(`${c.name} @flush: ${reasons.length ? `FAIL ${named(reasons)}` : 'ok'}`);
    if (reasons.length) failures++;
    flushed = 1;
  }
  if (FOCUS_CASES.includes(c.name)) {
    const { reasons, count } = await focusReasons(page, topMask, topArt);
    print(`${c.name} @focus: ${reasons.length ? `FAIL ${named(reasons)}` : `ok (${count} stops)`}`);
    if (reasons.length) failures++;
  }
  return { failures, control, flushed, readable: runs > 0 ? 1 : 0 };
}

// ---------------------------------------------------------------- self-check

/** The synthetic page: the kit frame as the 48 px nine-slice, a box on a knot and one in the centre. */
function selfCheckPage() {
  const data = fs.readFileSync(KIT_FRAME).toString('base64');
  return `<!doctype html><html><head><style>
    html, body { margin: 0; background: transparent; }
    .window { position: absolute; left: 40px; top: 40px; width: 280px; height: 240px; }
    .material-art { position: absolute; inset: 0; pointer-events: none; border: 0 solid transparent;
      border-image-source: url("data:image/png;base64,${data}"); border-image-slice: 224;
      border-image-width: 48px; border-image-outset: 8px; border-image-repeat: stretch; }
    .inner { position: absolute; inset: 0; overflow: hidden; }
    .centre { position: absolute; left: 120px; top: 100px; width: 40px; height: 40px; background: #0af; }
    .knot { position: absolute; left: 4px; top: 216px; width: 10px; height: 12px; background: #0af; }
  </style></head><body><div class="window"><div class="material-art"></div>
    <div class="inner"><div class="centre"></div><div class="knot"></div></div></div></body></html>`;
}

/**
 * The flush self-check's pages: the kit frame as the 48 px nine-slice over
 * the whole window. `margin` 12 (box 12 px in, 8 px out) must be reported
 * not flush; `margin` 0 with each outset just under the PNG's own padding
 * at 48 px must pass.
 */
function flushPage(margin, padding) {
  const data = fs.readFileSync(KIT_FRAME).toString('base64');
  const under = (pad) => Math.floor(((pad * 48) / 224) * 4) / 4;
  const outset = margin ? [8, 8, 8, 8] : [under(padding.top), under(padding.right), under(padding.bottom), under(padding.left)];
  return `<!doctype html><html><head><style>
    html, body { margin: 0; background: transparent; }
    .window { position: fixed; inset: ${margin}px; }
    .material-art { position: absolute; inset: 0; pointer-events: none; border: 0 solid transparent;
      border-image-source: url("data:image/png;base64,${data}"); border-image-slice: 224;
      border-image-width: 48px; border-image-outset: ${outset.map((o) => `${o}px`).join(' ')}; border-image-repeat: stretch; }
  </style></head><body><div class="window"><div class="material-art"></div></div></body></html>`;
}

/**
 * The readability self-check's page: over the well colour, a 9 px run, a
 * run in the disabled colour and a small-caps run (each must be reported)
 * and a compliant run (must pass).
 */
const TYPE_RUNS = { tiny: 'Nine pixel run', dim: 'Disabled colour run', caps: 'Small caps run', good: 'Compliant run' };
function typePage() {
  const run = (key, css) => `<p style="margin:0 0 8px;${css}">${TYPE_RUNS[key]}</p>`;
  return `<!doctype html><html><head><style>
    html, body { margin: 0; background: #090D10; }
    .window { position: absolute; inset: 0; padding: 20px; background: #090D10; color: #E4E6E7; font-family: Georgia, serif; font-size: 14px; }
  </style></head><body><div class="window">
    ${run('tiny', 'font-size:9px')}${run('dim', 'color:#535D64')}${run('caps', 'font-variant-caps:small-caps')}${run('good', '')}
  </div></body></html>`;
}

/** The self-check's reading of one page: the knot box seen, the centre box clear. */
function selfCheckReasons(m, mask) {
  const reasons = [];
  const labels = m.boxes.map((b) => b.label);
  for (const name of ['div.centre', 'div.knot']) if (!labels.includes(name)) reasons.push(`${name} not collected`);
  const hits = findOverlaps(mask, m.boxes);
  if (!hits.some((h) => h.label === 'div.knot')) reasons.push('the box on the knot was not reported');
  const others = hits.filter((h) => h.label !== 'div.knot');
  if (others.length) reasons.push(`art under ${named(others.map((h) => `${h.label} (${h.pixels}px)`))}`);
  return { reasons, knot: hits.find((h) => h.label === 'div.knot')?.pixels ?? 0 };
}

async function selfCheck(browser, padding, print) {
  let failures = 0;
  let control = true;
  let flushed = 0;
  let readable = 0;
  for (const scale of SCALES) {
    const context = await browser.newContext({ deviceScaleFactor: Number(scale), viewport: { width: 440, height: 400 } });
    const page = await context.newPage();
    try {
      // Flush: the 12 px margin must be reported, the flush frame must pass.
      const flush = [];
      for (const margin of [12, 0]) {
        await page.setContent(flushPage(margin, padding));
        await settle(page);
        flush.push(flushReasons(await takeMask(page), await page.evaluate(frameInPage), padding));
      }
      const [inset, edge] = flush;
      const flushProblems = [];
      if (!inset.length) flushProblems.push('the 12px margin was reported flush');
      if (edge.length) flushProblems.push(`the flush frame was reported: ${named(edge)}`);
      print(`self-check s${scale} @flush: ${flushProblems.length ? `FAIL ${flushProblems.join('; ')}` : `ok (12px margin not flush: ${named(inset)}; flush frame passes)`}`);
      if (flushProblems.length) failures++;
      flushed++;

      // The knot probe and the two boxes.
      await page.setContent(selfCheckPage());
      await settle(page);
      const mask = await takeMask(page);
      const probe = await page.evaluate(probeInPage, KNOT_PROBE);
      await page.evaluate(probeInPage, null);
      const knot = probe ? findOverlaps(mask, [probe]) : [];
      const detected = knot.length === 1;
      print(`self-check s${scale} @knot: ${detected ? `ok (${knot[0].pixels} art px under the probe)` : `FAIL control not detected (${controlMiss(probe)})`}`);
      if (!detected) {
        control = false;
        failures++;
      }
      const m = await page.evaluate(measureInPage);
      const { reasons, knot: pixels } = m.error ? { reasons: [m.error], knot: 0 } : selfCheckReasons(m, mask);
      print(`self-check s${scale} @boxes: ${reasons.length ? `FAIL ${reasons.join('; ')}` : `ok (knot box ${pixels} art px, centre box 0)`}`);
      if (reasons.length) failures++;

      // Readability: the three bad runs each reported for their own rule, the compliant one not at all.
      await page.setContent(typePage());
      await settle(page);
      const type = await readabilityReasons(page);
      const of = (key) => type.reasons.filter((r) => r.startsWith(`text "${TYPE_RUNS[key]}" `));
      const typeProblems = [];
      if (!of('tiny').some((r) => r.includes('readability'))) typeProblems.push('the 9px run was not reported');
      if (!of('dim').some((r) => r.includes('contrast'))) typeProblems.push('the #535D64 run was not reported');
      if (!of('caps').some((r) => r.includes('caps'))) typeProblems.push('the small-caps run was not reported');
      if (of('good').length) typeProblems.push(`the compliant run was reported: ${named(of('good'))}`);
      if (type.runs !== 4) typeProblems.push(`${type.runs} of 4 runs read`);
      print(`self-check s${scale} @readability: ${typeProblems.length ? `FAIL ${typeProblems.join('; ')}` : `ok (${named(type.reasons)}; compliant run passes)`}`);
      if (typeProblems.length) failures++;
      readable++;
    } finally {
      await context.close();
    }
  }
  return { cases: SCALES.length, failures, control, flushed, readable };
}

// ------------------------------------------------------- the base worktree

/** Runs a command, throwing with its output when it fails. */
function run(command, args, cwd) {
  const r = spawnSync(command, args, { cwd, encoding: 'utf8', shell: process.platform === 'win32' && command === 'npm' });
  if (r.status !== 0) throw new Error(`${command} ${args.join(' ')} failed (${r.status}): ${(r.stderr || r.stdout || String(r.error)).trim().split('\n').slice(-3).join(' | ')}`);
  return r.stdout;
}

/**
 * A scratch worktree of `ref` under the OS temp folder with the app's
 * dependencies installed; `remove` always takes it away again. Never a
 * junction to this tree's node_modules (`git worktree remove` would follow
 * it).
 */
function baseWorktree(ref, print) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'ch-corner-safety-base-'));
  fs.rmSync(dir, { recursive: true, force: true });
  const remove = () => {
    spawnSync('git', ['-C', REPO, 'worktree', 'remove', '--force', dir], { encoding: 'utf8' });
    spawnSync('git', ['-C', REPO, 'worktree', 'prune'], { encoding: 'utf8' });
    // A file the dev server's helper process still holds can refuse the
    // last delete on Windows; git has dropped the worktree by then, so a
    // detached process deletes the folder once this one has exited.
    try {
      fs.rmSync(dir, { recursive: true, force: true, maxRetries: 4, retryDelay: 250 });
    } catch {
      const script = `setTimeout(() => require('fs').rmSync(${JSON.stringify(dir)}, { recursive: true, force: true, maxRetries: 20, retryDelay: 500 }), 3000)`;
      spawn(process.execPath, ['-e', script], { detached: true, stdio: 'ignore' }).unref();
      console.log(`corner-safety: ${dir} is deleted after the run ends (git no longer lists it)`);
    }
  };
  try {
    run('git', ['-C', REPO, 'worktree', 'add', '--detach', dir, ref]);
    const app = path.join(dir, 'apps', 'crafting-helper');
    print(`corner-safety: base ${ref} in ${dir}; npm ci`);
    run('npm', ['ci', '--prefer-offline', '--no-audit', '--no-fund'], app);
    return { app, remove };
  } catch (error) {
    remove();
    throw error;
  }
}

// --------------------------------------------------------------------- serve

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

/** Vite from the served tree, so its config loads with its own plugins. */
async function loadVite(root) {
  try {
    return await import(pathToFileURL(createRequire(path.join(root, 'package.json')).resolve('vite')).href);
  } catch {
    return await import('vite');
  }
}

/** The tree's dev server on a free port, warmed so a dependency optimisation lands before any case. */
async function serve(root, browser) {
  const vite = await loadVite(root);
  const port = await freePort();
  const logger = vite.createLogger('warn');
  const error = logger.error.bind(logger);
  logger.error = (message, options) => {
    if (!/ResizeObserver loop/.test(String(message))) error(message, options);
  };
  const server = await vite.createServer({
    root,
    configFile: path.join(root, 'vite.config.ts'),
    customLogger: logger,
    clearScreen: false,
    server: { port, strictPort: true, host: '127.0.0.1', hmr: false },
  });
  await server.listen();
  const base = `http://127.0.0.1:${port}`;
  const context = await browser.newContext({ deviceScaleFactor: 1 });
  const page = await context.newPage();
  for (const url of ['/?state=craft-planning', '/settings.html?state=idle', '/overlay.html?state=overlay-active']) {
    await page.goto(base + url, { waitUntil: 'networkidle' });
  }
  await context.close();
  return { base, close: () => server.close() };
}

/** Caret, transitions and animations off, for a still frame to compare. */
function stillInPage() {
  const style = document.createElement('style');
  style.textContent = '*, *::before, *::after { caret-color: transparent !important; animation: none !important; transition: none !important; }';
  document.head.append(style);
  document.activeElement?.blur?.();
}

/** The legacy comparison: each case in the base tree and here with `theme=claude`. */
async function legacy(browser, args, print) {
  const tree = baseWorktree(args.legacyBase, print);
  let baseServer = null;
  let ours = null;
  try {
    baseServer = await serve(tree.app, browser);
    ours = await serve(args.root, browser);
    const context = await browser.newContext({ deviceScaleFactor: 1 });
    const page = await context.newPage();
    const declared = LEGACY_CASES.filter(([kind, url, w, h]) => !args.only || `${kind} ${url} ${w}x${h}`.includes(args.only));
    let differ = 0;
    let ran = 0;
    for (const [kind, url, w, h] of declared) {
      const name = `legacy ${kind} ${url} ${w}x${h}`;
      try {
        const images = [];
        for (const [server, query] of [[baseServer, url], [ours, `${url}&theme=claude`]]) {
          await open(page, server.base, query, READY_PLAIN[kind], w, h);
          await page.evaluate(stillInPage);
          // The Theme row is the one excused difference: neither tree draws it here.
          await page.evaluate(() => document.querySelectorAll('[data-theme-setting]').forEach((el) => el.remove()));
          await settle(page);
          images.push(await shot(page));
        }
        const pixels = pixelDiff(images[0], images[1]);
        print(`${name}: ${pixels === 0 ? 'identical' : `DIFFER ${pixels}`}`);
        if (pixels) differ++;
        ran++;
      } catch (error) {
        print(`${name}: DIFFER error: ${String(error?.message || error).split('\n')[0]}`);
        differ++;
      }
    }
    // The control: the same comparison must see Exile differ from the base,
    // or it can see nothing.
    let control = 0;
    {
      const [kind, url, w, h] = LEGACY_CASES[0];
      const images = [];
      for (const [server, query] of [[baseServer, url], [ours, `${url}&theme=exile`]]) {
        await open(page, server.base, query, READY_PLAIN[kind], w, h);
        await page.evaluate(stillInPage);
        await settle(page);
        images.push(await shot(page));
      }
      control = pixelDiff(images[0], images[1]);
      print(`legacy control ${kind} ${url} ${w}x${h} theme=exile: ${control > 0 ? `differs ${control} (control detected)` : 'FAIL identical, the comparison sees nothing'}`);
    }
    await context.close();
    if (ran !== declared.length) print(`legacy-pixels: ${declared.length - ran} of ${declared.length} declared cases did not run`);
    console.log(legacyLine(declared.length, differ));
    return differ === 0 && ran === declared.length && declared.length > 0 && control > 0 ? 0 : 1;
  } finally {
    if (ours) await ours.close();
    if (baseServer) await baseServer.close();
    tree.remove();
  }
}

/**
 * The reference crops: today's element for each requested-art id, from the
 * tree `--root` serves. The update mark shows only with a newer version
 * found, which no fixture holds: the runner sets it in the fixture module
 * as the dev server serves it, and fails when the mark still does not show.
 */
async function references(browser, root, dir, print) {
  const server = await serve(root, browser);
  try {
    fs.mkdirSync(dir, { recursive: true });
    const context = await browser.newContext({ deviceScaleFactor: 1 });
    const page = await context.newPage();
    for (const ref of REFERENCES) {
      await page.unrouteAll({ behavior: 'ignoreErrors' });
      if (ref.update) {
        const state = new URLSearchParams(ref.url.split('?')[1]).get('state');
        await page.route(`**/fixtures/views/${state}.json*`, async (route) => {
          const response = await route.fetch();
          const body = (await response.text())
            .replace(/(\\?"update_available\\?"\s*:\s*)null/g, (_m, key) => `${key}${key.includes('\\') ? `\\"${ref.update}\\"` : `"${ref.update}"`}`)
            .replace(/(\bupdate_available\s*=\s*)null/g, `$1"${ref.update}"`);
          await route.fulfill({ response, body });
        });
      }
      await open(page, server.base, ref.url, READY_PLAIN[ref.kind], ref.width, ref.height);
      await page.evaluate(stillInPage);
      const el = await page.waitForSelector(ref.selector, { timeout: 15000 });
      await settle(page);
      await el.screenshot({ path: path.join(dir, `${ref.id}.png`) });
      print(`reference ${ref.id}: ${ref.selector} in ${ref.url}`);
    }
    await context.close();
    console.log(`corner-safety: ${REFERENCES.length} references in ${dir}`);
    return 0;
  } finally {
    await server.close();
  }
}

// --------------------------------------------------------------------- main

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const print = (line) => console.log(line);
  // Playwright hides every scrollbar in headless mode; the app shows them,
  // and the scrollbar-as-content check needs them drawn.
  const browser = await chromium.launch({ channel: 'msedge', headless: true, ignoreDefaultArgs: ['--hide-scrollbars'] });
  let tree = null;
  let server = null;
  try {
    // The kit frame's own transparent padding, which alone may leave the window.
    const padding = artPadding(alphaMask(decodePng(new Uint8Array(fs.readFileSync(KIT_FRAME)), inflate)));
    if (args.selfCheck) {
      const r = await selfCheck(browser, padding, print);
      console.log(summaryLine(r.cases, r.failures, r.control, r.flushed, r.readable));
      return r.failures === 0 && r.control ? 0 : 1;
    }
    if (args.legacyBase) return await legacy(browser, args, print);
    if (args.rootRef) {
      tree = baseWorktree(args.rootRef, print);
      args.root = tree.app;
    }
    if (args.references) return await references(browser, args.root, args.references, print);

    server = await serve(args.root, browser);
    const declared = declareCases(args.full).filter((c) => !args.only || c.name.includes(args.only));
    if (declared.length === 0) {
      console.log(`corner-safety: no case matches --only ${args.only}`);
      return 1;
    }
    let failures = 0;
    let ran = 0;
    let flushed = 0;
    let readable = 0;
    const controlled = new Map();
    for (const scale of SCALES) {
      const cases = declared.filter((c) => c.scale === scale);
      if (!cases.length) continue;
      const context = await browser.newContext({ deviceScaleFactor: Number(scale), viewport: { width: 960, height: 680 } });
      const page = await context.newPage();
      try {
        for (const c of cases) {
          // The control runs on the first main case at each scale (the overlay has no frame).
          const withControl = !controlled.has(scale) && c.kind === 'main';
          try {
            const r = await runCase(page, server.base, c, withControl, padding, print);
            failures += r.failures;
            flushed += r.flushed;
            readable += r.readable;
            if (withControl) controlled.set(scale, r.control === true);
            ran++;
          } catch (error) {
            print(`${c.name} @top: FAIL error: ${String(error?.message || error).split('\n')[0]}`);
            failures++;
          }
        }
      } finally {
        await context.close();
      }
    }
    // The control must have been seen at every scale the run covered.
    const control = [...new Set(declared.map((c) => c.scale))].every((s) => controlled.get(s) === true);
    if (ran !== declared.length) print(`corner-safety: ${declared.length - ran} of ${declared.length} declared cases did not run`);

    if (args.screenshots) {
      fs.mkdirSync(args.screenshots, { recursive: true });
      const context = await browser.newContext({ deviceScaleFactor: 1 });
      const page = await context.newPage();
      for (const [name, kind, url, width, height] of SCREENSHOTS) {
        await open(page, server.base, url, READY_PLAIN[kind], width, height);
        await page.evaluate(stillInPage);
        await settle(page);
        await page.screenshot({ path: path.join(args.screenshots, `${name}.png`), omitBackground: kind === 'overlay' });
      }
      await context.close();
      console.log(`corner-safety: ${SCREENSHOTS.length} screenshots in ${args.screenshots}`);
    }

    console.log(summaryLine(declared.length, failures, control, flushed, readable));
    return failures === 0 && ran === declared.length && control ? 0 : 1;
  } finally {
    if (server) await server.close();
    await browser.close();
    if (tree) tree.remove();
  }
}

main().then(
  (code) => process.exit(code),
  (error) => {
    console.error(`corner-safety: ${error instanceof UsageError ? error.message : error?.stack || error}`);
    process.exit(2);
  },
);
