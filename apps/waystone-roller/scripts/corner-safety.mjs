// `npm run corner-safety`: proves that nothing Waystone Roller's Exile theme
// paints as content (text, marks, icons, controls, dividers, scrollbars,
// focus rings) lands on the frame artwork, in both windows, at every size,
// device scale, text size and scroll position of the browser previews, by
// measuring rendered pixels rather than comparing rectangles. Adapted from
// the Quest Tracker's runner (`apps/quest-tracker/scripts/corner-safety.mjs`,
// whose header is the specification); its play, edit and Next up checks have
// no counterpart here.
//
// It starts this app's Vite dev server on a free OS-assigned port (never
// 1421, which `tauri dev` owns), drives the installed Microsoft Edge
// headless through `playwright-core` (no browser download, scrollbars not
// hidden), and serves the existing previews with `theme=exile`. A case's
// scale is the device scale factor (these windows have no in-app zoom; a
// display scale changes device pixels, not layout), and every screenshot is
// taken in CSS pixels. For each case:
//
//   - renders the artwork alone (every element but `.material-art` hidden,
//     transparent page, screenshot without background): alpha >= 64 is art;
//   - collects the client rects of every non-empty text node and the boxes of
//     svg, img, form controls, buttons, scrollbars and elements with a visible
//     border or background inside the page, each clipped by every clipping
//     ancestor and the viewport;
//   - fails when any clipped box covers an art pixel, on sideways overflow,
//     and when a box is never fully visible at any scroll position (taller or
//     wider than its scroller, or cut off with nothing to scroll it).
//
// The flush check (every case): the frame meets the window edge. The
// rendered art (alpha >= 64) comes within 2 px of each viewport edge, and no
// art pixel is cut off by the window, computed from `.material-art`'s box,
// its border-image outset and width per side, and the kit PNG's own
// transparent padding (decoded here).
//
// The readability check (every case, every scroll position): each text run
// on screen renders at least 12 px; is at least 4.5:1 against the worst
// pixel behind it, read from a screenshot with all text made transparent (a
// disabled control's text is exempt); has no `font-variant-caps`, no caps
// face (Cinzel) below 13 px, a weight of at least 400, and tracking of at
// most 0.05em below 13 px (0.1em above).
//
// Focus (the main window with `ok-any` and the Settings window `idle`, at
// the default size and scale 1): Tab through every focusable; each focused
// element with its 2 px ring is fully visible and art-free.
//
// A control runs at every scale: a probe placed over the bottom-left knot of
// the rendered frame must be reported, or the run fails. `--self-check` runs
// only synthetic pages that draw the kit frame PNG as the 48 px nine-slice:
// one with a box on a knot (must be reported) and one in the centre (must
// not), one 12 px in from the window that must be reported not flush and a
// flush one that must pass, and a page of text runs (9 px, a low-contrast
// colour and small caps must each be reported; a compliant one must pass).
// It needs no app, so it shows the instrument works while the app changes.
//
// `--legacy-base <ref>`: the Claude theme renders today's pixels. The runner
// makes a scratch worktree of `<ref>` under the OS temp folder, runs
// `npm ci --prefer-offline` in its `apps/waystone-roller`, serves it beside
// this tree, renders each legacy case in both (the base ignores `theme`;
// this tree gets `theme=claude`) at device scale 1, fonts ready, two
// animation frames, caret and animations off, and compares the decoded
// pixels. Before a Settings screenshot it removes `[data-theme-setting]`
// from both trees (the base has none), so only the new Theme section is
// excused. It always removes the worktree (`git worktree remove --force`,
// then `git worktree prune`), even on failure.
//
// Flags:
//   --root <app dir>        serve another checkout's app
//   --only <substring>      run only the cases whose name contains it
//   --self-check            the synthetic pages only
//   --screenshots <dir>     also save the six review images there
//   --references <dir>      save today's element of each requested-art id
//                           there, from `--root`, or from the base worktree
//                           of `--legacy-base` when no `--root` is given
//   --legacy-base <ref>     the pixel comparison of Claude with `<ref>`
//                           (only that comparison, with --references and
//                           --screenshots when given)
//   --full                  also text 200% at every scale (not required to pass)
//
// Output: one line per case and position, `ok` or `FAIL <reasons>`, then
// `corner-safety: <n> cases, <f> failures, control detected, flush checked
// on <k> cases, readability checked on <r> cases`. Exit 0 only when nothing
// failed, every declared case ran and the control was detected. With
// `--legacy-base`: one line per case, `identical` or `DIFFER <pixels>`, then
// `legacy-pixels: <n> cases, <d> differ`; exit 0 only when `d` is 0 and every
// declared case ran.

import { execFileSync, spawn } from 'node:child_process';
import fs from 'node:fs';
import net from 'node:net';
import os from 'node:os';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';
import zlib from 'node:zlib';
import { chromium } from 'playwright-core';
import {
  alphaMask, artPadding, artPixels, clipRect, contrastReasons, decodePng, differingPixels, findOverlaps, flushReasons, grow,
  legacyLine, parseColour, summaryLine, typeReasons, worstContrast,
} from './corner-safety-core.mjs';

const APP = path.resolve(fileURLToPath(new URL('..', import.meta.url)));
const REPO = path.resolve(APP, '../..');
const APP_IN_REPO = path.relative(REPO, APP).split(path.sep).join('/');
const KIT_FRAME = path.resolve(APP, '../../docs/quest-tracker/design/steel-masonry-kit/assets/frame-neutral-steel.png');

/** The app's `devUrl` port, which a run never takes. */
const DEV_PORT = 1421;
const SCALES = ['0.75', '1', '1.33', '2'];
/** The main window's default and minimum sizes (`tauri.conf.json`). */
const MAIN_SIZES = [[800, 860], [560, 480]];
/** The Settings window's size (`src-tauri/src/lib.rs` SETTINGS_WIDTH, SETTINGS_HEIGHT). */
const SETTINGS_SIZE = [420, 364];
/** Every view fixture (`fixtures/views/`). */
const MAIN_VIEWS = ['empty', 'ok-any', 'ok-all', 'too-long', 'hotkey-taken', 'unmatchable'];
/** The Settings window's states: the fixture under `fixtures/app-settings/`. */
const SETTINGS_STATES = ['idle', 'available', 'install-downloading', 'install-failed'];
/**
 * The probe on the bottom-left knot, as fractions of the frame's drawn
 * bottom-left corner block (border-image width, from its outset edge): the
 * kit PNG's corner slice is solid artwork there (a 48px block, measured).
 */
const KNOT_PROBE = { x: 12 / 48, y: 16 / 48, width: 10 / 48, height: 12 / 48 };
/** How many offenders one line names before it counts the rest. */
const NAMED = 4;
/** The page roots, in the order a probe looks for one. */
const ROOTS = ['.app', '.window', '.panel'];

// ---------------------------------------------------------------- arguments

/** A usage mistake: reported in one line, exit 2. */
class UsageError extends Error {}

function parseArgs(argv) {
  // `npm --prefix <app> run` starts the script in the app folder; a relative
  // path means the folder the command was typed in (`INIT_CWD`).
  const from = process.env.INIT_CWD || process.cwd();
  const args = { root: APP, rootGiven: false, only: null, screenshots: null, references: null, legacyBase: null, selfCheck: false, full: false };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    const value = () => {
      if (i + 1 >= argv.length) throw new UsageError(`${a} needs a value`);
      return argv[++i];
    };
    if (a === '--root') { args.root = path.resolve(from, value()); args.rootGiven = true; }
    else if (a === '--only') args.only = value();
    else if (a === '--screenshots') args.screenshots = path.resolve(from, value());
    else if (a === '--references') args.references = path.resolve(from, value());
    else if (a === '--legacy-base') args.legacyBase = value();
    else if (a === '--self-check') args.selfCheck = true;
    else if (a === '--full') args.full = true;
    else {
      throw new UsageError(
        `unknown argument ${a} (flags: --root <app dir>, --only <substring>, --screenshots <dir>, --references <dir>, --legacy-base <ref>, --self-check, --full)`,
      );
    }
  }
  if (!args.selfCheck && !fs.existsSync(path.join(args.root, 'vite.config.ts'))) {
    throw new UsageError(`--root ${args.root} has no vite.config.ts`);
  }
  if (args.references && !args.rootGiven && !args.legacyBase) {
    throw new UsageError('--references needs --root <base app dir> or --legacy-base <ref> to read today\'s elements from');
  }
  return args;
}

// -------------------------------------------------------------------- cases

const mainUrl = (view, theme) => `/?view=${view}${theme ? `&theme=${theme}` : ''}`;
const settingsUrl = (state, theme) => `/settings.html?state=${state}${theme ? `&theme=${theme}` : ''}`;

/** Every declared case, in run order. */
function declareCases(full) {
  const cases = [];
  const textSets = (scale) => (scale === '1' || full ? [100, 200] : [100]);
  for (const scale of SCALES) {
    for (const text of textSets(scale)) {
      for (const [w, h] of MAIN_SIZES) {
        for (const view of MAIN_VIEWS) {
          cases.push({
            name: `main ${view} ${w}x${h} s${scale} t${text}`,
            kind: 'main', width: w, height: h, scale, text,
            url: mainUrl(view, 'exile'),
            focus: view === 'ok-any' && w === 800 && scale === '1' && text === 100,
          });
        }
      }
      const [w, h] = SETTINGS_SIZE;
      for (const state of SETTINGS_STATES) {
        cases.push({
          name: `settings ${state} ${w}x${h} s${scale} t${text}`,
          kind: 'settings', width: w, height: h, scale, text,
          url: settingsUrl(state, 'exile'),
          focus: state === 'idle' && scale === '1' && text === 100,
        });
      }
    }
  }
  return cases;
}

/** The legacy comparison's cases: Claude in this tree against the base tree. */
function declareLegacy() {
  const cases = [];
  for (const view of MAIN_VIEWS) cases.push({ name: `legacy main ${view} 800x860`, kind: 'main', width: 800, height: 860, view });
  cases.push({ name: 'legacy main too-long 560x480', kind: 'main', width: 560, height: 480, view: 'too-long' });
  for (const state of SETTINGS_STATES) cases.push({ name: `legacy settings ${state} 420x364`, kind: 'settings', width: 420, height: 364, state });
  return cases.map((c) => ({
    ...c,
    baseUrl: c.kind === 'main' ? mainUrl(c.view, null) : settingsUrl(c.state, null),
    oursUrl: c.kind === 'main' ? mainUrl(c.view, 'claude') : settingsUrl(c.state, 'claude'),
  }));
}

/** The six review images `--screenshots` writes, at device scale 1. */
const SCREENSHOTS = [
  ['exile-main-default', mainUrl('ok-any', 'exile'), 800, 860],
  ['exile-main-min', mainUrl('too-long', 'exile'), 560, 480],
  ['exile-settings-idle', settingsUrl('idle', 'exile'), 420, 364],
  ['exile-settings-downloading', settingsUrl('install-downloading', 'exile'), 420, 364],
  ['claude-main-default', mainUrl('ok-any', 'claude'), 800, 860],
  ['claude-settings-idle', settingsUrl('idle', 'claude'), 420, 364],
];

/**
 * Today's element of each requested-art id (`--references`), read from the
 * base tree in its own look: the page, the element, and how much of its
 * surroundings to keep around it (CSS px).
 */
const REFERENCES = [
  ['wr-mark-want', mainUrl('ok-any', null), 800, 860, '.mod[data-choice="want"] .choice-mark', 10],
  ['wr-mark-avoid', mainUrl('ok-any', null), 800, 860, '.mod[data-choice="avoid"] .choice-mark', 10],
  ['wr-update-dot', mainUrl('ok-all', null), 800, 860, '.update-mark', 6],
];

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
 * Tags the page's scrollers (`data-cs-scroller`) and scrolls them:
 * `target` is null to list them, or `{ index, where }` to scroll one
 * (`top`, `mid`, `end`) with every other at the top.
 */
function scrollInPage(target) {
  const root = document.querySelector('.app') || document.querySelector('.window') || document.querySelector('.panel');
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
 * screenshot holds in CSS pixels); `offsetWidth`, `clientWidth` and computed
 * lengths are unzoomed CSS px, so each is scaled by the element's own ratio
 * of the two (1 here, with no CSS zoom).
 */
function measureInPage() {
  const root = document.querySelector('.app') || document.querySelector('.window') || document.querySelector('.panel');
  if (!root) return { error: 'no .app, .window or .panel rendered' };
  const EXCLUDED = '.material-art, [data-cs-probe]';
  const WELL_ART = /masonry-charcoal/;
  const CONTROLS = 'img, input, select, textarea, button, [role="button"], [role="switch"], [role="slider"], [role="checkbox"], [role="radio"], [role="tab"], [role="combobox"], [role="option"]';
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
      let clip = own;
      for (const c of clipsFor(scroller, false)) clip = clip && intersect(clip, c);
      // A scroller that runs off screen never shows what scrolls to its end.
      if (clip && !cutScrollers.has(scroller) && (clip.top > own.top + 0.5 || clip.bottom < own.bottom - 0.5)) {
        cutScrollers.add(scroller);
        unreachable.push(`scroller ${describe(scroller).split(' ')[0]} is cut off (shows ${Math.round(clip.top)}-${Math.round(clip.bottom)} of ${Math.round(own.top)}-${Math.round(own.bottom)})`);
      }
      // A horizontal cut inside the scroller is left alone, as above: text
      // ending in an ellipsis inside its own clipping box paints only that
      // box's width. Its height is held to the scroller whole.
      let inner = box.rect;
      for (let a = self ? el : el.parentElement; inner && a && a !== scroller; a = a.parentElement) {
        if (clips(getComputedStyle(a))) inner = intersect(inner, paddingBox(a));
      }
      const w = inner ? inner.right - inner.left : 0;
      const h = rect.bottom - rect.top;
      if (!clip) unreachable.push(`${label} in a hidden scroller`);
      else if (h > clip.bottom - clip.top + 0.5 || w > clip.right - clip.left + 0.5) {
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

  // Scrollbars of the page's scrollers.
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

  // Sideways overflow, of the page and of every scroller in it.
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
  return { boxes, problems, unreachable, sideways, art: art ? plain(art.getBoundingClientRect()) : null };
}

/**
 * Every text run the page shows at this scroll position, for the
 * readability check: its computed type, its colour with the opacity of
 * everything above it, and its line rects clipped to what is on screen. A
 * run clipped away here is left for the position that shows it.
 */
function textRunsInPage() {
  const root = document.querySelector('.app') || document.querySelector('.window') || document.querySelector('.panel');
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
  const root = document.querySelector('.app') || document.querySelector('.window') || document.querySelector('.panel');
  if (!art || !root) return null;
  // Beside the page's content, not in a scroller, so no scroller's clip can
  // hide it: the control asks whether art under a painted box is seen.
  const host = art.parentElement;
  const probe = document.createElement('div');
  probe.setAttribute('data-cs-probe', '');
  probe.style.cssText = 'position:absolute;left:0;top:0;width:10px;height:10px;background:#f0f;z-index:99;pointer-events:none';
  host.append(probe);
  // The bottom-left corner block as the frame really draws it: its
  // border-image width and outset (computed px) in viewport px.
  const a = art.getBoundingClientRect();
  const cs = getComputedStyle(art);
  const zoom = art.offsetWidth > 0 ? a.width / art.offsetWidth : 1;
  const length = (v, fallback) => (v.endsWith('px') ? parseFloat(v) : fallback) * zoom;
  const corner = length(cs.borderImageWidth.split(' ')[0], 48);
  const outsets = cs.borderImageOutset.split(' ');
  // Bottom and left: the third and fourth values of a list of up to four.
  const bottomOutset = length(outsets[2] ?? outsets[0], 0);
  const leftOutset = length(outsets[3] ?? outsets[1] ?? outsets[0], 0);
  const block = { left: a.left - leftOutset, bottom: a.bottom + bottomOutset };
  const want = {
    left: block.left + corner * options.x,
    top: block.bottom - corner + corner * options.y,
    width: corner * options.width,
    height: corner * options.height,
  };
  // Correct the placement by what the browser actually did (transforms and
  // positioned ancestors all move an absolute box).
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
    const ecs = getComputedStyle(el);
    if (ecs.overflowX !== 'visible' || ecs.overflowY !== 'visible') {
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
  // A hidden native radio covers its segment exactly; the segment draws the ring.
  const ringEl = el.matches('.segmented input') ? el.closest('label') || el : el;
  const r = ringEl.getBoundingClientRect();
  const zoom = ringEl.offsetWidth > 0 ? r.width / ringEl.offsetWidth : (ringEl.currentCSSZoom || 1);
  const clips = [];
  for (let a = ringEl.parentElement; a && a !== document.body && a !== document.documentElement; a = a.parentElement) {
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
  const label = `${el.tagName.toLowerCase()}${classes} "${(el.getAttribute('aria-label') || el.textContent || el.getAttribute('value') || '').trim().replace(/\s+/g, ' ').slice(0, 24)}"`;
  const art = document.querySelector('.material-art')?.getBoundingClientRect();
  const ring = getComputedStyle(ringEl.matches('label') ? ringEl : el);
  return {
    key: el.getAttribute('data-cs-focus'),
    label,
    rect: { left: r.left, top: r.top, right: r.right, bottom: r.bottom },
    clips,
    zoom,
    ringWidth: parseFloat(ring.outlineWidth) || 0,
    ringStyle: ring.outlineStyle,
    art: art ? { left: art.left, top: art.top, right: art.right, bottom: art.bottom } : null,
  };
}

/**
 * The frame as the page draws it, for the flush check: `.material-art`'s
 * box and its border-image outset, width and slice per side, in viewport px;
 * a bare number is a multiple of the (zero) border width.
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

/** Removes the Theme section (`[data-theme-setting]`), which the base tree does not have. */
function dropThemeSettingInPage() {
  document.querySelectorAll('[data-theme-setting]').forEach((el) => el.remove());
}

// ------------------------------------------------------------- node side

const inflate = (data) => new Uint8Array(zlib.inflateSync(data));

async function settle(page) {
  await page.evaluate(settleInPage);
}

/** The artwork mask of the page as it stands, in CSS px. */
async function takeMask(page) {
  await page.evaluate(maskStyleInPage, true);
  await settle(page);
  const png = await page.screenshot({ omitBackground: true, type: 'png', scale: 'css' });
  await page.evaluate(maskStyleInPage, false);
  await settle(page);
  return alphaMask(decodePng(new Uint8Array(png), inflate));
}

/**
 * The page as it stands with every text run transparent: what lies behind
 * the text, in CSS px. Not `omitBackground`: where the page lets the window
 * through, the browser's white stands in, the worst case for light text.
 */
async function takeBackground(page) {
  await page.evaluate(hideTextInPage, true);
  await settle(page);
  const png = await page.screenshot({ type: 'png', scale: 'css' });
  await page.evaluate(hideTextInPage, false);
  await settle(page);
  return decodePng(new Uint8Array(png), inflate);
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

/** Tab through the window: each focused element, with its 2 px ring, fully visible and art-free. */
async function focusReasons(page, topMask, topArt) {
  await page.evaluate(() => {
    document.activeElement?.blur?.();
    document.querySelectorAll('[data-cs-focus]').forEach((el) => el.removeAttribute('data-cs-focus'));
  });
  await page.evaluate(scrollInPage, null);
  const reasons = [];
  const seen = new Set();
  let count = 0;
  for (let i = 0; i < 200; i++) {
    await page.keyboard.press('Tab');
    await settle(page);
    const f = await page.evaluate(focusedInPage);
    if (!f || seen.has(f.key)) break;
    seen.add(f.key);
    count++;
    if (f.ringStyle === 'none' || f.ringWidth < 2 - 0.05) reasons.push(`${f.label} has no 2px focus ring (${f.ringStyle} ${f.ringWidth}px)`);
    const ring = grow(f.rect, 2 * f.zoom);
    const shown = clipRect(ring, f.clips);
    if (!shown) reasons.push(`${f.label} hidden when focused`);
    else {
      const cut = [['left', shown.left - ring.left], ['top', shown.top - ring.top], ['right', ring.right - shown.right], ['bottom', ring.bottom - shown.bottom]]
        .filter(([, by]) => by > 0.5).map(([side, by]) => `${side} ${Math.round(by * 10) / 10}px`);
      if (cut.length) reasons.push(`${f.label} clipped when focused (${cut.join(', ')})`);
    }
    const mask = sameRect(f.art, topArt) ? topMask : await takeMask(page);
    const pixels = shown ? artPixels(mask, shown) : 0;
    if (pixels > 0) reasons.push(`${f.label} focused over art (${pixels}px)`);
  }
  if (count === 0) reasons.push('no element took focus');
  return { reasons, count };
}

/** The page's root once its first view has rendered (the Exile frame with it, when asked). */
async function waitReady(page, kind, framed) {
  const root = kind === 'main' ? '.app[data-state]' : '.window[data-state]:not([data-state="none"])';
  await page.waitForSelector(framed ? `${root} .material-art` : root, { timeout: 15000 });
  await settle(page);
}

/**
 * One case: every scroll position, the flush check, focus when asked, and
 * the control when asked. `padding` is the frame PNG's own.
 */
async function runCase(page, base, c, withControl, padding, print) {
  await page.setViewportSize({ width: c.width, height: c.height });
  await page.goto(base + c.url, { waitUntil: 'load' });
  await waitReady(page, c.kind, true);
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
  {
    const reasons = flushReasons(topMask, await page.evaluate(frameInPage), padding);
    print(`${c.name} @flush: ${reasons.length ? `FAIL ${named(reasons)}` : 'ok'}`);
    if (reasons.length) failures++;
  }
  if (c.focus) {
    const { reasons, count } = await focusReasons(page, topMask, topArt);
    print(`${c.name} @focus: ${reasons.length ? `FAIL ${named(reasons)}` : `ok (${count} stops)`}`);
    if (reasons.length) failures++;
  }
  return { failures, control, flushed: 1, readable: runs > 0 ? 1 : 0 };
}

// ---------------------------------------------------------------- self-check

/** The synthetic page: the kit frame as the 48/8 nine-slice, a box on a knot and one in the centre. */
function selfCheckPage() {
  const data = fs.readFileSync(KIT_FRAME).toString('base64');
  return `<!doctype html><html><head><style>
    html, body { margin: 0; background: transparent; }
    .panel { position: absolute; left: 40px; top: 40px; width: 280px; height: 240px; }
    .material-art { position: absolute; inset: 0; pointer-events: none; border: 0 solid transparent;
      border-image-source: url("data:image/png;base64,${data}"); border-image-slice: 224;
      border-image-width: 48px; border-image-outset: 8px; border-image-repeat: stretch; }
    .content { position: absolute; inset: 0; overflow: hidden; }
    .centre { position: absolute; left: 120px; top: 100px; width: 40px; height: 40px; background: #0af; }
    .knot { position: absolute; left: 4px; top: 216px; width: 10px; height: 12px; background: #0af; }
  </style></head><body><div class="panel"><div class="material-art"></div>
    <div class="content"><div class="centre"></div><div class="knot"></div></div></div></body></html>`;
}

/**
 * The flush self-check's pages: the kit frame as the 48 px nine-slice over
 * the whole window. `margin` 12 puts the box 12 px in with an 8 px outset,
 * which must be reported not flush; `margin` 0 puts it on the window edge
 * with each side's outset just under the PNG's own padding, which must pass.
 */
function flushPage(margin, padding) {
  const data = fs.readFileSync(KIT_FRAME).toString('base64');
  const under = (pad) => Math.floor((pad * 48) / 224 * 4) / 4;
  const outset = margin
    ? [8, 8, 8, 8]
    : [under(padding.top), under(padding.right), under(padding.bottom), under(padding.left)];
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
    .app { position: absolute; inset: 0; padding: 20px; background: #090D10; color: #E4E6E7; font-family: Georgia, serif; font-size: 14px; }
  </style></head><body><div class="app">
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

async function selfCheck(pages, padding, print) {
  let failures = 0;
  let control = true;
  let flushed = 0;
  let readable = 0;
  for (const scale of SCALES) {
    const page = await pages(scale);
    // The flush check: the 12 px margin must be reported, the flush frame must pass.
    await page.setViewportSize({ width: 420, height: 364 });
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

    await page.setViewportSize({ width: 364, height: 384 });
    await page.setContent(selfCheckPage());
    await settle(page);
    const mask = await takeMask(page);
    const probe = await page.evaluate(probeInPage, KNOT_PROBE);
    await page.evaluate(probeInPage, null);
    const knot = probe ? findOverlaps(mask, [probe]) : [];
    const detected = knot.length === 1;
    print(`self-check s${scale} @knot: ${detected ? `ok (${knot[0].pixels} art px under the probe)` : `FAIL control not detected (${controlMiss(probe)})`}`);
    if (!detected) { control = false; failures++; }
    // The same reading the app's cases get: a content box on the knot must
    // be reported, the one in the centre must not.
    const m = await page.evaluate(measureInPage);
    const { reasons, knot: pixels } = selfCheckReasons(m, mask);
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
  }
  return { cases: SCALES.length, failures, control, flushed, readable };
}

// ---------------------------------------------------------- legacy and crops

/** Today's look, compared pixel for pixel with Claude in this tree. */
async function legacy(page, oursBase, baseBase, print) {
  const cases = declareLegacy();
  let differ = 0;
  let ran = 0;
  const shot = async (url, c) => {
    await page.setViewportSize({ width: c.width, height: c.height });
    await page.goto(url, { waitUntil: 'load' });
    await waitReady(page, c.kind, false);
    if (c.kind === 'settings') await page.evaluate(dropThemeSettingInPage);
    await settle(page);
    await settle(page);
    return decodePng(new Uint8Array(await page.screenshot({ type: 'png', animations: 'disabled', caret: 'hide' })), inflate);
  };
  for (const c of cases) {
    try {
      const base = await shot(baseBase + c.baseUrl, c);
      const ours = await shot(oursBase + c.oursUrl, c);
      const n = differingPixels(base, ours);
      print(`${c.name}: ${n === 0 ? 'identical' : `DIFFER ${n}`}`);
      if (n !== 0) differ++;
      ran++;
    } catch (error) {
      print(`${c.name}: DIFFER error: ${String(error?.message || error).split('\n')[0]}`);
      differ++;
    }
  }
  if (ran !== cases.length) print(`legacy-pixels: ${cases.length - ran} of ${cases.length} declared cases did not run`);
  return { cases: cases.length, differ, ran };
}

/** Today's element of each requested-art id, cropped with a little of its surroundings. */
async function references(page, base, dir, print) {
  fs.mkdirSync(dir, { recursive: true });
  for (const [id, url, width, height, selector, margin] of REFERENCES) {
    await page.setViewportSize({ width, height });
    await page.goto(base + url, { waitUntil: 'load' });
    await waitReady(page, 'main', false);
    const el = await page.waitForSelector(selector, { timeout: 15000 });
    await el.scrollIntoViewIfNeeded();
    await settle(page);
    const box = await el.boundingBox();
    if (!box) throw new Error(`${id}: ${selector} has no box`);
    const clip = {
      x: Math.max(0, box.x - margin),
      y: Math.max(0, box.y - margin),
      width: Math.min(width, box.x + box.width + margin) - Math.max(0, box.x - margin),
      height: Math.min(height, box.y + box.height + margin) - Math.max(0, box.y - margin),
    };
    await page.screenshot({ path: path.join(dir, `${id}.png`), clip, animations: 'disabled', caret: 'hide' });
    print(`reference ${id}: ${selector} ${Math.round(box.width)}x${Math.round(box.height)} css px at scale 1`);
  }
  print(`corner-safety: ${REFERENCES.length} references in ${dir}`);
}

async function screenshots(page, base, dir) {
  fs.mkdirSync(dir, { recursive: true });
  for (const [name, url, width, height] of SCREENSHOTS) {
    await page.setViewportSize({ width, height });
    await page.goto(base + url, { waitUntil: 'load' });
    await waitReady(page, url.startsWith('/settings') ? 'settings' : 'main', name.startsWith('exile'));
    await settle(page);
    await page.screenshot({ path: path.join(dir, `${name}.png`), animations: 'disabled', caret: 'hide' });
  }
  console.log(`corner-safety: ${SCREENSHOTS.length} screenshots in ${dir}`);
}

/**
 * A scratch worktree of `ref` under the OS temp folder with its app's
 * dependencies installed; `remove` takes it away again (never through a
 * junction: `node_modules` is the worktree's own). Its dev server runs in a
 * child process (`serveChild`), so no native module of it stays loaded in
 * this one and Windows lets the folder go.
 */
async function baseWorktree(ref) {
  const git = (...a) => execFileSync('git', a, { cwd: REPO, stdio: ['ignore', 'pipe', 'pipe'], encoding: 'utf8' });
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'wr-corner-safety-base-'));
  const remove = async () => {
    try { git('worktree', 'remove', '--force', dir); } catch { /* removed below */ }
    for (let i = 0; i < 10 && fs.existsSync(dir); i++) {
      try { fs.rmSync(dir, { recursive: true, force: true }); } catch { await new Promise((r) => setTimeout(r, 500)); }
    }
    try { git('worktree', 'prune'); } catch { /* best effort */ }
    if (fs.existsSync(dir)) console.log(`corner-safety: could not remove the base worktree ${dir}; delete it by hand`);
  };
  try {
    git('worktree', 'add', '--detach', dir, ref);
    const app = path.join(dir, APP_IN_REPO);
    const ci = ['ci', '--prefer-offline', '--no-audit', '--no-fund'];
    const options = { cwd: app, stdio: ['ignore', 'ignore', 'pipe'] };
    // Under `npm run`, the npm that started us, through this Node (no shell);
    // run directly, the `npm` on the path.
    if (process.env.npm_execpath) execFileSync(process.execPath, [process.env.npm_execpath, ...ci], options);
    else execFileSync(process.platform === 'win32' ? 'npm.cmd' : 'npm', ci, { ...options, shell: process.platform === 'win32' });
    return { app, remove };
  } catch (error) {
    await remove();
    throw error;
  }
}

/**
 * The app at `root` served by its own Vite in a child process, on a free
 * port; resolves to its base URL and a `close` that ends the process.
 */
async function serveChild(root) {
  const port = await freePort();
  const bin = path.join(root, 'node_modules', 'vite', 'bin', 'vite.js');
  const child = spawn(process.execPath, [bin, '--port', String(port), '--strictPort', '--host', '127.0.0.1', '--logLevel', 'warn', '--clearScreen', 'false'], {
    cwd: root,
    stdio: ['ignore', 'ignore', 'pipe'],
  });
  let stderr = '';
  child.stderr.on('data', (d) => { stderr += d; });
  const exited = new Promise((resolve) => child.once('exit', resolve));
  const close = async () => {
    if (child.exitCode === null) child.kill();
    await exited;
  };
  const base = `http://127.0.0.1:${port}`;
  for (let i = 0; ; i++) {
    if (child.exitCode !== null) throw new Error(`the base tree's dev server stopped: ${stderr.trim().split('\n')[0]}`);
    try {
      if ((await fetch(`${base}/`)).ok) break;
    } catch { /* not listening yet */ }
    if (i >= 120) { await close(); throw new Error('the base tree\'s dev server did not start within a minute'); }
    await new Promise((r) => setTimeout(r, 500));
  }
  return { base, close };
}

// --------------------------------------------------------------------- main

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

/** Vite from the served checkout, so its config loads with its own plugins. */
async function loadVite(root) {
  try {
    return await import(pathToFileURL(createRequire(path.join(root, 'package.json')).resolve('vite')).href);
  } catch {
    return await import('vite');
  }
}

/** A dev server for the app at `root` on a free port; resolves to its base URL and server. */
async function serve(root) {
  const vite = await loadVite(root);
  const port = await freePort();
  const logger = vite.createLogger('warn');
  const error = logger.error.bind(logger);
  // The browser's benign "ResizeObserver loop" report says nothing about a case.
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
  return { base: `http://127.0.0.1:${port}`, server };
}

/** Loads both pages once, so a dependency optimisation reload lands before the cases. */
async function warm(page, base) {
  for (const url of ['/?view=ok-any', '/settings.html?state=idle']) await page.goto(base + url, { waitUntil: 'networkidle' });
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const print = (line) => console.log(line);
  // Playwright hides every scrollbar in headless mode; the app shows them,
  // and the scrollbar-as-content checks need them drawn.
  const browser = await chromium.launch({ channel: 'msedge', headless: true, ignoreDefaultArgs: ['--hide-scrollbars'] });
  const contexts = new Map();
  /** One page per device scale, in its own context. */
  const pageAt = async (scale) => {
    if (!contexts.has(scale)) {
      const context = await browser.newContext({ deviceScaleFactor: Number(scale), viewport: { width: 800, height: 860 } });
      contexts.set(scale, await context.newPage());
    }
    return contexts.get(scale);
  };
  const servers = [];
  let worktree = null;
  let baseServer = null;
  try {
    // The kit frame's own transparent padding, which alone may leave the window.
    const padding = artPadding(alphaMask(decodePng(new Uint8Array(fs.readFileSync(KIT_FRAME)), inflate)));
    if (args.selfCheck) {
      const r = await selfCheck(pageAt, padding, print);
      console.log(summaryLine(r.cases, r.failures, r.control, r.flushed, r.readable));
      return r.failures === 0 && r.control ? 0 : 1;
    }

    if (args.legacyBase) {
      worktree = await baseWorktree(args.legacyBase);
      const ours = await serve(APP);
      servers.push(ours.server);
      baseServer = await serveChild(worktree.app);
      const base = baseServer;
      const page = await pageAt('1');
      await warm(page, ours.base);
      await warm(page, base.base);
      if (args.references && !args.rootGiven) {
        const sharp = await pageAt('2');
        await warm(sharp, base.base);
        await references(sharp, base.base, args.references, print);
      }
      const r = await legacy(page, ours.base, base.base, print);
      if (args.screenshots) await screenshots(page, ours.base, args.screenshots);
      console.log(legacyLine(r.cases, r.differ));
      return r.differ === 0 && r.ran === r.cases ? 0 : 1;
    }

    const served = await serve(args.root);
    servers.push(served.server);
    const base = served.base;
    if (args.references) {
      const page = await pageAt('2');
      await warm(page, base);
      await references(page, base, args.references, print);
    }

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
    const warmed = new Set();
    for (const c of declared) {
      const page = await pageAt(c.scale);
      if (!warmed.has(c.scale)) {
        await warm(page, base);
        warmed.add(c.scale);
      }
      const withControl = !controlled.has(c.scale);
      try {
        const r = await runCase(page, base, c, withControl, padding, print);
        failures += r.failures;
        flushed += r.flushed;
        readable += r.readable;
        if (withControl) controlled.set(c.scale, r.control === true);
        ran++;
      } catch (error) {
        print(`${c.name} @top: FAIL error: ${String(error?.message || error).split('\n')[0]}`);
        failures++;
      }
    }
    // The control must have been seen at every scale the run covered.
    const control = [...new Set(declared.map((c) => c.scale))].every((s) => controlled.get(s) === true);
    if (ran !== declared.length) print(`corner-safety: ${declared.length - ran} of ${declared.length} declared cases did not run`);

    if (args.screenshots) await screenshots(await pageAt('1'), base, args.screenshots);

    console.log(summaryLine(declared.length, failures, control, flushed, readable));
    return failures === 0 && ran === declared.length && control ? 0 : 1;
  } finally {
    await browser.close();
    for (const server of servers) await server.close();
    await baseServer?.close();
    await worktree?.remove();
  }
}

main().then(
  (code) => process.exit(code),
  (error) => {
    console.error(`corner-safety: ${error instanceof UsageError ? error.message : error?.stack || error}`);
    process.exit(2);
  },
);
