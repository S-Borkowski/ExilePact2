// `npm run corner-safety`: proves that nothing the Exile theme paints as
// content (text, status marks, icons, controls, dividers, scrollbars) lands
// on the frame artwork, at every surface, size, scale, text size and scroll
// position of the browser previews, by measuring rendered pixels rather than
// comparing rectangles.
//
// It starts this app's Vite dev server on a free port (never 1420, which
// `tauri dev` owns), drives the installed Microsoft Edge headless through
// `playwright-core` (no browser download), and for each case:
//
//   - renders the artwork alone (every element but `.material-art` hidden,
//     transparent page, screenshot without background): alpha >= 64 is art;
//   - collects the client rects of every non-empty text node and the boxes of
//     svg, img, `.status-mark`, `.icon`, form controls, buttons, scrollbars
//     and elements with a visible border or background inside the panel,
//     each clipped by every clipping ancestor and the viewport;
//   - fails when any clipped box covers an art pixel, and on the other
//     checks below (sideways overflow, a box taller or wider than its
//     scroller, controls in play, the edit drag strip, focus, fixed chrome).
//
// One exemption: a button whose text wraps (an edit-mode quest row at a
// large scale or text size) may be taller than its scroller. It is then
// checked line by line, not whole: each of its text lines and marks must fit
// the scroller, and when focused only its ring's left and right edges must be
// unclipped.
//
// The flush check (every case): the frame meets the window edge. The
// rendered art (alpha >= 64) comes within 2 px of each viewport edge, and no
// art pixel is cut off by the window, computed from `.material-art`'s box,
// its border-image outset and width per side, and the kit PNG's own
// transparent padding (decoded here). In edit, each of the eight resize
// handles is what the page hits 2 px inside the middle of its window edge or
// inside its corner.
//
// The readability check (every case, every scroll position; owner, Live 2):
// each text run on screen renders at least 14 px for quest names and titles,
// 13 px for objectives, tips and detail text, and 12 px for everything else
// (computed size times the page zoom); is at least 4.5:1 against the worst
// pixel behind it, read from a screenshot with all text made transparent (a
// disabled control's text is exempt); has no `font-variant-caps`, no caps
// face (Cinzel) below 13 px, a weight of at least 400, and tracking of at
// most 0.05em below 13 px (0.1em above).
//
// The play-scrollbar check (owner, Live 2: play is click-through): in play
// every overflowing scroller shows no scrollbar and still scrolls; in edit
// and both Settings surfaces each keeps one. Playwright hides scrollbars in
// headless mode, so the browser is started without that switch.
//
// The sections check (owner, Live 3: which quests are worth it and which
// optional must read at a glance), on every overlay case showing two or
// more Next up sections (runs of one worth): each section is labelled by a
// visible heading (`aria-labelledby`), no separator sits under a section's
// last quest, every section after the first has exactly one `.section-break`
// above its heading, a separator between two quests of one section draws no
// diamond and is at most half a break's height, and is inset (owner, after
// Live 4): it starts at or right of its row's quest name and is narrower
// than any section break, else `separator not inset`; and no heading touches
// a separator or a break. The overlay matrix includes the dev-only
// three-section view, `/?frame=next-up-sections` (`src/preview.ts`).
//
// A control runs at every scale: a probe placed over the bottom-left knot of
// the rendered frame must be reported, or the run fails. `--self-check` runs
// only synthetic pages that draw the kit frame PNG as the 48 px nine-slice:
// one with a probe on a knot and one in the centre, one 12 px in from the
// window that must be reported not flush, and one flush that must pass; plus
// a page of four text runs (9 px, the disabled colour, small caps: each must
// be reported; a compliant one must pass) and a play-marked scroller with
// its bar shown (must be reported) and hidden (must pass); and a two-section
// Next up laid out as before the sections (must be reported), as sections
// with a full-width line between two quests of one section (must be
// reported not inset) and with that line inset to the quest name (must
// pass). It needs no app,
// so it shows the instrument works while the app is being changed.
//
// Flags:
//   --root <app dir>     serve another checkout's app (the baseline)
//   --only <substring>   run only the cases whose name contains it
//   --screenshots <dir>  also save the eight review images there
//   --self-check         the synthetic page only
//   --full               also text 200% at every scale (not required to pass)
//
// Output: one line per case and position, `ok` or `FAIL <reasons>`, then
// `corner-safety: <n> cases, <f> failures, control detected, flush checked
// on <k> cases, readability checked on <r> cases, play scrollbar checked on
// <p> cases, sections checked on <s> cases` (r: cases with text runs read;
// p: play cases with an overflowing scroller; s: cases with two or more Next
// up sections). Exit 0 only when nothing failed, every declared
// case ran and the control was detected.

import fs from 'node:fs';
import net from 'node:net';
import path from 'node:path';
import { createRequire } from 'node:module';
import { fileURLToPath, pathToFileURL } from 'node:url';
import zlib from 'node:zlib';
import { chromium } from 'playwright-core';
import {
  alphaMask, artPadding, artPixels, clipRect, contrastReasons, decodePng, findOverlaps, flushReasons, grow, parseColour,
  scrollbarReasons, sectionReasons, summaryLine, typeReasons, worstContrast,
} from './corner-safety-core.mjs';

const APP = path.resolve(fileURLToPath(new URL('..', import.meta.url)));
const KIT_FRAME = path.resolve(APP, '../../docs/quest-tracker/design/steel-masonry-kit/assets/frame-neutral-steel.png');

const SCALES = ['0.75', '1', '1.33', '2'];
const OVERLAY_SIZES = [[364, 384], [320, 240]];
const SETTINGS_SIZE = [420, 328];
/** The overlay surfaces: frame id and mode (null: the fixture's own mode). */
const OVERLAY_SURFACES = [
  ['panel-next-up', 'play'], ['panel-next-up', 'edit'],
  // The dev-only three-section Next up view (`src/preview.ts`).
  ['next-up-sections', 'play'], ['next-up-sections', 'edit'],
  ['panel-campaign', 'play'], ['panel-campaign', 'edit'],
  ['quest-detail', 'play'], ['quest-detail', 'edit'],
  ['character-picker', 'edit'], ['settings', 'edit'],
  ['empty-act-done', null], ['empty-no-character', null], ['empty-no-log', null], ['empty-waiting', null],
];
/** The ordinary Settings window's states: query and name. */
const SETTINGS_STATES = [['state=idle', 'idle'], ['state=available', 'available'], ['install=downloading', 'install=downloading'], ['state=failed', 'failed']];
/** What a play surface may not contain. */
const INTERACTIVE = 'button, input, select, textarea, a[href], [role="button"], [tabindex]';
/**
 * The probe on the bottom-left knot, as fractions of the frame's drawn
 * bottom-left corner block (border-image width, from its outset edge): the
 * kit PNG's corner slice is solid artwork there (a 48px block, measured).
 */
const KNOT_PROBE = { x: 12 / 48, y: 16 / 48, width: 10 / 48, height: 12 / 48 };
/** How many offenders one line names before it counts the rest. */
const NAMED = 4;
/**
 * The readability check's roles, by the run's nearest matching ancestor:
 * quest names and frame or window titles are `title`, objective and step
 * text, tips and detail text `body`, every other run `meta`.
 */
const TEXT_ROLES = {
  title: '.row[data-quest] .name, .detail .head .name, .empty .title, .tracker-title, .tracker-act, .frame-title, #settings-title',
  body: '.row[data-quest] .step, .row[data-quest] .tip, .detail .step-title, .detail .step-text, .detail .step-tip, .detail .reason, .empty .text',
};

// ---------------------------------------------------------------- arguments

/** A usage mistake: reported in one line, exit 2. */
class UsageError extends Error {}

function parseArgs(argv) {
  // `npm --prefix <app> run` starts the script in the app folder; a relative
  // path means the folder the command was typed in (`INIT_CWD`).
  const from = process.env.INIT_CWD || process.cwd();
  const args = { root: APP, only: null, screenshots: null, selfCheck: false, full: false };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    const value = () => {
      if (i + 1 >= argv.length) throw new UsageError(`${a} needs a value`);
      return argv[++i];
    };
    if (a === '--root') args.root = path.resolve(from, value());
    else if (a === '--only') args.only = value();
    else if (a === '--screenshots') args.screenshots = path.resolve(from, value());
    else if (a === '--self-check') args.selfCheck = true;
    else if (a === '--full') args.full = true;
    else throw new UsageError(`unknown argument ${a} (flags: --root <app dir>, --only <substring>, --screenshots <dir>, --self-check, --full)`);
  }
  if (!args.selfCheck && !fs.existsSync(path.join(args.root, 'vite.config.ts'))) {
    throw new UsageError(`--root ${args.root} has no vite.config.ts`);
  }
  return args;
}

// -------------------------------------------------------------------- cases

/** Every declared case, in run order. */
function declareCases(full) {
  const cases = [];
  const textSets = (scale) => (scale === '1' || full ? [100, 200] : [100]);
  for (const [w, h] of OVERLAY_SIZES) {
    for (const scale of SCALES) {
      for (const text of textSets(scale)) {
        for (const [frame, mode] of OVERLAY_SURFACES) {
          cases.push({
            name: `overlay ${frame}${mode ? ` ${mode}` : ''} ${w}x${h} s${scale} t${text}`,
            kind: 'overlay', frame, width: w, height: h, scale, text,
            url: `/?frame=${frame}${mode ? `&mode=${mode}` : ''}&scale=${scale}&theme=exile`,
            // Default panels keep a fixed header and footer (compact density).
            fixedChrome: w === 364 && h === 384 && scale === '1' && text === 100 &&
              ((frame === 'panel-next-up' && mode === 'play') || (frame === 'panel-campaign' && mode === 'edit') || frame === 'quest-detail'),
          });
        }
      }
    }
  }
  const [w, h] = SETTINGS_SIZE;
  for (const scale of SCALES) {
    for (const text of textSets(scale)) {
      for (const [query, state] of SETTINGS_STATES) {
        cases.push({
          name: `settings ${state} ${w}x${h} s${scale} t${text}`,
          kind: 'settings', width: w, height: h, scale, text, fixedChrome: false,
          url: `/settings.html?${query}&scale=${scale}&theme=exile`,
        });
      }
    }
  }
  return cases;
}

/** The eight review images `--screenshots` writes. */
const SCREENSHOTS = [
  ['play-default', '/?frame=panel-next-up&mode=play&scale=1&theme=exile', 364, 384],
  ['edit-all-default', '/?frame=panel-campaign&mode=edit&scale=1&theme=exile', 364, 384],
  ['detail-min', '/?frame=quest-detail&mode=edit&scale=1&theme=exile', 320, 240],
  ['all-min', '/?frame=panel-campaign&mode=edit&scale=1&theme=exile', 320, 240],
  ['overlay-settings-default', '/?frame=settings&mode=edit&scale=1&theme=exile', 364, 384],
  ['ordinary-settings', '/settings.html?state=idle&scale=1&theme=exile', 420, 328],
  ['claude-play-default', '/?frame=panel-next-up&mode=play&scale=1&theme=claude', 364, 384],
  ['sections-min', '/?frame=next-up-sections&mode=play&scale=1&theme=exile', 320, 240],
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
 * Tags the panel's scrollers (`data-cs-scroller`) and scrolls them:
 * `positions` is null to list them, or `{ index, where }` to scroll one
 * (`top`, `mid`, `end`) with every other at the top.
 */
function scrollInPage(target) {
  const root = document.querySelector('.panel') || document.querySelector('.window');
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
function measureInPage(options) {
  const root = document.querySelector('.panel') || document.querySelector('.window');
  if (!root) return { error: 'no .panel or .window rendered' };
  const EXCLUDED = '.material-art, .frame-drag-region, .resize-edge, [data-cs-probe]';
  const WELL_ART = /masonry-charcoal|charcoal-stone-well/;
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
   * A button holding text that wraps onto two or more lines (an edit-mode
   * quest row). When it is taller than its scroller it can never be seen
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
      let clip = own;
      for (const c of clipsFor(scroller, false)) clip = clip && intersect(clip, c);
      // A scroller that runs off screen never shows what scrolls to its end.
      if (clip && !cutScrollers.has(scroller) && (clip.top > own.top + 0.5 || clip.bottom < own.bottom - 0.5)) {
        cutScrollers.add(scroller);
        unreachable.push(`scroller ${describe(scroller).split(' ')[0]} is cut off (shows ${Math.round(clip.top)}-${Math.round(clip.bottom)} of ${Math.round(own.top)}-${Math.round(own.bottom)})`);
      }
      const w = rect.right - rect.left;
      const h = rect.bottom - rect.top;
      if (!clip) unreachable.push(`${label} in a hidden scroller`);
      else if ((h > clip.bottom - clip.top + 0.5 && (self || !wrapsText(el))) || w > clip.right - clip.left + 0.5) {
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

  const mode = document.querySelector('.overlay')?.getAttribute('data-mode') || null;
  const interactive = [...document.querySelectorAll(options.interactive)].filter((el) => !el.closest('.resize-edge')).map(describe);
  const dragEl = document.querySelector('.frame-drag-region');
  let drag = null;
  if (dragEl) {
    const r = dragEl.getBoundingClientRect();
    const hit = document.elementFromPoint((r.left + r.right) / 2, (r.top + r.bottom) / 2);
    drag = { hit: !!hit && (hit === dragEl || dragEl.contains(hit)), what: hit ? describe(hit) : 'nothing' };
  }
  const chromeOf = (el) => {
    if (!el) return null;
    const rect = plain(el.getBoundingClientRect());
    let shown = rect;
    for (const c of clipsFor(el, false)) shown = shown && intersect(shown, c);
    return { rect, shown };
  };
  const art = document.querySelector('.material-art');
  return {
    boxes,
    problems,
    unreachable,
    sideways,
    mode,
    interactive,
    drag,
    hasDragRegion: !!dragEl,
    header: chromeOf(root.querySelector('.header')),
    footer: chromeOf(root.querySelector('.play-footer, .back-to-play')),
    art: art ? plain(art.getBoundingClientRect()) : null,
  };
}

/**
 * Every text run the panel shows at this scroll position, for the
 * readability check: its role (by `options.title` / `options.body`, else
 * meta), its computed type, its colour with the opacity of everything above
 * it, and its line rects clipped to what is on screen. A run clipped away
 * here is left for the position that shows it.
 */
function textRunsInPage(options) {
  const root = document.querySelector('.panel') || document.querySelector('.window');
  if (!root) return [];
  const EXCLUDED = '.material-art, .frame-drag-region, .resize-edge, [data-cs-probe], .sr-only';
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
      role: parent.closest(options.title) ? 'title' : parent.closest(options.body) ? 'body' : 'meta',
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

/**
 * The panel's overflowing scrollers, for the play-scrollbar check: each
 * one's scrollbar thickness (CSS px) and whether setting `scrollTop` moves
 * it. Each is put back where it was.
 */
function scrollersInPage() {
  const root = document.querySelector('.panel') || document.querySelector('.window');
  if (!root) return [];
  const out = [];
  for (const el of [root, ...root.querySelectorAll('*')]) {
    if (!(el instanceof HTMLElement) || el.closest('.material-art, .resize-edge')) continue;
    const cs = getComputedStyle(el);
    if (!/(auto|scroll)/.test(cs.overflowY) || el.scrollHeight <= el.clientHeight + 1) continue;
    const thickness = el.offsetWidth - el.clientWidth - el.clientLeft - (parseFloat(cs.borderRightWidth) || 0);
    const was = el.scrollTop;
    el.scrollTop = was > 0 ? 0 : el.scrollHeight;
    const scrollable = el.scrollTop !== was;
    el.scrollTop = was;
    const classes = [...el.classList].slice(0, 2).map((c) => `.${c}`).join('');
    out.push({ label: `${el.tagName.toLowerCase()}${classes}`, thickness, scrollable });
  }
  return out;
}

/**
 * The Next up sections as drawn, for the sections check: the quest rows of
 * a Next up panel in order, grouped by runs of one `data-worth` (the order
 * the engine sends), whatever elements wrap them. For each: the heading its
 * section element's `aria-labelledby` names, if visible; the separator under
 * its last row, if one is displayed; the separators under its other rows,
 * with whether each draws the centre diamond and where that row's visible
 * quest name starts (null when it shows none); and the visible
 * `.section-break` elements between the previous run's last row and its
 * first. Null on a page that is not a Next up panel.
 */
function sectionsInPage() {
  const body = document.querySelector('.overlay[data-frame="panel-next-up"] .panel .body');
  if (!body) return null;
  const visible = (el) => {
    if (!el) return null;
    const cs = getComputedStyle(el);
    const r = el.getBoundingClientRect();
    if (cs.display === 'none' || cs.visibility === 'hidden' || r.width <= 0 || r.height <= 0) return null;
    return { left: r.left, top: r.top, right: r.right, bottom: r.bottom };
  };
  const separatorOf = (row) => row.querySelector(':scope > .quest-divider');
  const runs = [];
  for (const row of body.querySelectorAll('.row[data-quest]')) {
    const worth = row.getAttribute('data-worth') || '';
    const last = runs[runs.length - 1];
    if (last && last.worth === worth) last.rows.push(row);
    else runs.push({ worth, rows: [row] });
  }
  const breaks = [...body.querySelectorAll('.section-break')];
  const before = (a, b) => !!(a.compareDocumentPosition(b) & Node.DOCUMENT_POSITION_FOLLOWING);
  return runs.map((run, i) => {
    const first = run.rows[0];
    const lastRow = run.rows[run.rows.length - 1];
    const section = first.closest('section');
    const id = section && body.contains(section) ? section.getAttribute('aria-labelledby') : null;
    const label = id ? document.getElementById(id) : null;
    const heading = label && section.contains(label) ? visible(label) : null;
    const separators = run.rows.slice(0, -1).flatMap((row) => {
      const sep = separatorOf(row);
      const box = visible(sep);
      if (!box) return [];
      const after = getComputedStyle(sep, '::before');
      const name = visible(row.querySelector('.name'));
      return [{ box, diamond: after.content !== 'none' && after.display !== 'none', nameLeft: name ? name.left : null }];
    });
    const previous = i > 0 ? runs[i - 1].rows[runs[i - 1].rows.length - 1] : null;
    return {
      worth: run.worth,
      heading,
      lastSeparator: visible(separatorOf(lastRow)),
      separators,
      breaks: previous ? breaks.filter((b) => before(previous, b) && before(b, first)).map(visible).filter(Boolean) : [],
    };
  });
}

/** Places the knot probe and returns its box and clips, or removes it. */
function probeInPage(options) {
  document.querySelectorAll('[data-cs-probe]').forEach((el) => el.remove());
  if (!options) return null;
  const art = document.querySelector('.material-art');
  const root = document.querySelector('.panel') || document.querySelector('.window');
  if (!art || !root) return null;
  // Beside the panel, not in it, so the panel's own clip cannot hide it:
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
  // A button whose text wraps (an edit-mode quest row) and whose ring is
  // taller than the window its clips leave can never show that ring whole:
  // only its left and right edges are held to it. Its lines are checked one
  // by one at the scroll positions.
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

/** Edit's resize handles on the window's outer edge: each one the page hits 2 px inside its edge's middle or its corner. */
function resizeHitsInPage() {
  const w = window.innerWidth;
  const h = window.innerHeight;
  const points = [
    ['North', w / 2, 2], ['South', w / 2, h - 2], ['West', 2, h / 2], ['East', w - 2, h / 2],
    ['NorthWest', 2, 2], ['NorthEast', w - 2, 2], ['SouthWest', 2, h - 2], ['SouthEast', w - 2, h - 2],
  ];
  const misses = [];
  for (const [direction, x, y] of points) {
    const hit = document.elementFromPoint(x, y);
    if (hit?.closest(`.resize-edge[data-resize="${direction}"]`)) continue;
    const what = hit ? `${hit.tagName.toLowerCase()}${[...hit.classList].slice(0, 2).map((c) => `.${c}`).join('')}${hit.getAttribute('data-resize') ? `[${hit.getAttribute('data-resize')}]` : ''}` : 'nothing';
    misses.push(`${direction} handle not hit at ${Math.round(x)},${Math.round(y)} (${what})`);
  }
  return misses;
}

// ------------------------------------------------------------- node side

const inflate = (data) => new Uint8Array(zlib.inflateSync(data));

async function settle(page) {
  await page.evaluate(settleInPage);
}

/** The artwork mask of the page as it stands. */
async function takeMask(page) {
  await page.evaluate(maskStyleInPage, true);
  await settle(page);
  const png = await page.screenshot({ omitBackground: true, type: 'png' });
  await page.evaluate(maskStyleInPage, false);
  await settle(page);
  return alphaMask(decodePng(new Uint8Array(png), inflate));
}

/**
 * The page as it stands with every text run transparent: what lies behind
 * the text. Not `omitBackground`: where the panel lets the page through, the
 * browser's white stands in for whatever is behind the window, the worst
 * case for light text.
 */
async function takeBackground(page) {
  await page.evaluate(hideTextInPage, true);
  await settle(page);
  const png = await page.screenshot({ type: 'png' });
  await page.evaluate(hideTextInPage, false);
  await settle(page);
  return decodePng(new Uint8Array(png), inflate);
}

/**
 * The readability check at one scroll position: each run on screen against
 * its role's size floor, the caps, weight and tracking rules, and 4.5:1
 * against the worst pixel behind it (a disabled control's text is exempt
 * from contrast, as WCAG exempts it). Each reason names its run.
 */
async function readabilityReasons(page) {
  const runs = await page.evaluate(textRunsInPage, TEXT_ROLES);
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
function positionReasons(c, m, mask) {
  const reasons = [];
  const overlaps = findOverlaps(mask, m.boxes);
  if (overlaps.length) reasons.push(`art under ${overlaps.length} boxes: ${named(overlaps.map((o) => `${o.label} (${o.pixels}px)`))}`);
  if (m.sideways.length) reasons.push(`sideways overflow: ${named(m.sideways)}`);
  if (m.unreachable.length) reasons.push(`never fully visible: ${named(m.unreachable)}`);
  if (m.problems.length) reasons.push(named(m.problems));
  if (c.kind === 'overlay') {
    if (m.mode === 'play') {
      if (m.interactive.length) reasons.push(`play has controls: ${named(m.interactive)}`);
      if (m.hasDragRegion) reasons.push('play has a drag region');
    } else if (m.mode === 'edit') {
      if (!m.drag) reasons.push('edit has no .frame-drag-region');
      else if (!m.drag.hit) reasons.push(`drag region covered at its centre by ${m.drag.what}`);
    }
  }
  return reasons;
}

/** The fixed-chrome rule over a case's positions: header and footer never move or clip. */
function chromeReasons(measures) {
  const reasons = [];
  for (const part of ['header', 'footer']) {
    const first = measures[0]?.[part];
    if (!first) { reasons.push(`no ${part}`); continue; }
    for (const m of measures) {
      const at = m[part];
      if (!at || !sameRect(at.rect, first.rect)) { reasons.push(`${part} moves with the scroll`); break; }
      if (!at.shown || !sameRect(at.shown, at.rect)) { reasons.push(`${part} is clipped`); break; }
    }
  }
  return reasons;
}

/** Tab through the surface: each focused element, with a 2px outline, fully visible and art-free. */
async function focusReasons(page, topMask, topArt) {
  await page.evaluate(() => {
    document.activeElement?.blur?.();
    document.querySelectorAll('[data-cs-focus]').forEach((el) => el.removeAttribute('data-cs-focus'));
  });
  await page.evaluate(scrollInPage, null);
  const reasons = [];
  const seen = new Set();
  let count = 0;
  for (let i = 0; i < 80; i++) {
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
      // A focused row taller than its scroller is held to its left and right edges only.
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

/**
 * One case: every scroll position, the fixed-chrome rule, the flush check,
 * focus, and the control when asked. `padding` is the frame PNG's own.
 */
async function runCase(page, base, c, withControl, padding, print) {
  await page.setViewportSize({ width: c.width, height: c.height });
  await page.goto(base + c.url, { waitUntil: 'load' });
  const ready = c.kind === 'overlay' ? '.overlay[data-frame]:not([data-frame="none"]) .material-art' : '.window[data-state]:not([data-state="none"]) .material-art';
  await page.waitForSelector(ready, { timeout: 15000 });
  await settle(page);
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
  const measures = [];
  const unreadable = new Set();
  let runs = 0;
  let played = 0;
  let sectioned = 0;
  for (const p of positions) {
    await page.evaluate(scrollInPage, p.target);
    await settle(page);
    const mask = await takeMask(page);
    const m = await page.evaluate(measureInPage, { interactive: INTERACTIVE });
    if (m.error) throw new Error(m.error);
    measures.push(m);
    if (!topMask) {
      topMask = mask;
      topArt = m.art;
      // Play: no scrollbar, still scrollable; edit and both Settings keep theirs.
      const scrollers = await page.evaluate(scrollersInPage);
      if (scrollers.length) {
        const mode = c.kind === 'settings' ? 'edit' : m.mode;
        const reasons = scrollbarReasons(mode, scrollers);
        if (mode === 'play') played = 1;
        print(`${c.name} @scrollbar: ${reasons.length ? `FAIL ${named(reasons)}` : `ok (${mode}, ${scrollers.length} scrollers)`}`);
        if (reasons.length) failures++;
      }
      // Next up sections: the layout does not move with the scroll, so the top position stands for all.
      const sections = c.kind === 'overlay' ? await page.evaluate(sectionsInPage) : null;
      if (sections && sections.length >= 2) {
        const reasons = sectionReasons(sections);
        sectioned = 1;
        print(`${c.name} @sections: ${reasons.length ? `FAIL ${named(reasons)}` : `ok (${sections.map((s) => s.worth).join(', ')})`}`);
        if (reasons.length) failures++;
      }
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
    const reasons = positionReasons(c, m, mask);
    print(`${c.name} @${p.label}: ${reasons.length ? `FAIL ${reasons.join('; ')}` : 'ok'}`);
    if (reasons.length) failures++;
  }
  print(`${c.name} @readability: ${unreadable.size ? `FAIL ${named([...unreadable])}` : `ok (${runs} runs)`}`);
  if (unreadable.size) failures++;
  if (c.fixedChrome) {
    const reasons = chromeReasons(measures);
    print(`${c.name} @fixed-chrome: ${reasons.length ? `FAIL ${reasons.join('; ')}` : 'ok'}`);
    if (reasons.length) failures++;
  }
  // Flush: the frame does not move with the scroll, so the top position's mask stands for all.
  {
    const reasons = flushReasons(topMask, await page.evaluate(frameInPage), padding);
    if (c.kind === 'overlay' && measures[0].mode === 'edit') reasons.push(...(await page.evaluate(resizeHitsInPage)));
    print(`${c.name} @flush: ${reasons.length ? `FAIL ${named(reasons)}` : 'ok'}`);
    if (reasons.length) failures++;
  }
  if (c.kind === 'settings' || measures[0].mode === 'edit') {
    const { reasons, count } = await focusReasons(page, topMask, topArt);
    print(`${c.name} @focus: ${reasons.length ? `FAIL ${named(reasons)}` : `ok (${count} stops)`}`);
    if (reasons.length) failures++;
  }
  return { failures, control, flushed: 1, readable: runs > 0 ? 1 : 0, played, sectioned };
}

// ---------------------------------------------------------------- self-check

/** The synthetic page: the kit frame as the 48/8 nine-slice at a constant on-screen size. */
function selfCheckPage(scale) {
  const data = fs.readFileSync(KIT_FRAME).toString('base64');
  const z = Number(scale);
  return `<!doctype html><html style="zoom:${scale}"><head><style>
    html, body { margin: 0; background: transparent; }
    .panel-shadow { position: absolute; left: ${40 / z}px; top: ${40 / z}px; width: ${280 / z}px; height: ${240 / z}px; }
    .material-art { position: absolute; inset: 0; pointer-events: none; border: 0 solid transparent;
      border-image-source: url("data:image/png;base64,${data}"); border-image-slice: 224;
      border-image-width: ${48 / z}px; border-image-outset: ${8 / z}px; border-image-repeat: stretch; }
    .panel { position: absolute; inset: 0; overflow: hidden; }
    .centre { position: absolute; left: ${120 / z}px; top: ${100 / z}px; width: ${40 / z}px; height: ${40 / z}px; background: #0af; }
    .knot { position: absolute; left: ${4 / z}px; top: ${216 / z}px; width: ${10 / z}px; height: ${12 / z}px; background: #0af; }
  </style></head><body><div class="panel-shadow"><div class="material-art"></div>
    <div class="panel"><div class="centre"></div><div class="knot"></div></div></div></body></html>`;
}

/**
 * The flush self-check's pages: the kit frame as the 48 px nine-slice over
 * the whole window, at a constant on-screen size. `margin` 12 is the old
 * Exile geometry (box 12 px in, 8 px out), which must be reported not
 * flush; `margin` 0 puts the box on the window edge with each side's outset
 * just under the PNG's own padding at 48 px, which must pass.
 */
function flushPage(scale, margin, padding) {
  const data = fs.readFileSync(KIT_FRAME).toString('base64');
  const z = Number(scale);
  const under = (pad) => Math.floor((pad * 48) / 224 * 4) / 4;
  const outset = margin
    ? [8, 8, 8, 8]
    : [under(padding.top), under(padding.right), under(padding.bottom), under(padding.left)];
  return `<!doctype html><html style="zoom:${scale}"><head><style>
    html, body { margin: 0; background: transparent; }
    .panel-shadow { position: fixed; inset: ${margin / z}px; }
    .material-art { position: absolute; inset: 0; pointer-events: none; border: 0 solid transparent;
      border-image-source: url("data:image/png;base64,${data}"); border-image-slice: 224;
      border-image-width: ${48 / z}px; border-image-outset: ${outset.map((o) => `${o / z}px`).join(' ')}; border-image-repeat: stretch; }
  </style></head><body><div class="panel-shadow"><div class="material-art"></div></div></body></html>`;
}

/**
 * The readability self-check's page, at a constant rendered size: over the
 * well colour, a 9 px run, a run in the disabled colour and a small-caps run
 * (each must be reported) and a compliant run (must pass).
 */
const TYPE_RUNS = { tiny: 'Nine pixel run', dim: 'Disabled colour run', caps: 'Small caps run', good: 'Compliant run' };
function typePage(scale) {
  const z = Number(scale);
  const run = (key, css) => `<p style="margin:0 0 ${8 / z}px;${css}">${TYPE_RUNS[key]}</p>`;
  return `<!doctype html><html style="zoom:${scale}"><head><style>
    html, body { margin: 0; background: #090D10; }
    .panel { position: absolute; inset: 0; padding: ${20 / z}px; background: #090D10; color: #E4E6E7; font-family: Georgia, serif; font-size: ${14 / z}px; }
  </style></head><body><div class="panel">
    ${run('tiny', `font-size:${9 / z}px`)}${run('dim', 'color:#535D64')}${run('caps', 'font-variant-caps:small-caps')}${run('good', '')}
  </div></body></html>`;
}

/** The play-scrollbar self-check's page: a play-marked overflowing scroller, its 8 px bar shown or hidden. */
function scrollbarPage(hidden) {
  return `<!doctype html><html><head><style>
    html, body { margin: 0; }
    .body { height: 120px; overflow-y: auto; ${hidden ? 'scrollbar-width: none;' : ''} }
    .body::-webkit-scrollbar { width: 8px; }
  </style></head><body><div class="overlay" data-mode="play"><div class="panel"><div class="body"><div style="height:600px">Tall</div></div></div></div></body></html>`;
}

/**
 * The sections self-check's pages, each row a status-mark-sized box then
 * its `.name`: a two-section Next up laid out as before Fix 3 (`flat`: an
 * unlabelled section, the full diamond divider under every quest including
 * a group's last, no break), which must be reported; the Fix 3 layout
 * (`full`: labelled sections, one break above the second heading, a 1 px
 * line the row's full width between two quests of one section), which must
 * be reported as not inset; and that line inset to the quest name
 * (`inset`), which must pass.
 *
 * @param {'flat' | 'full' | 'inset'} layout
 */
function sectionsPage(layout) {
  const compliant = layout !== 'flat';
  const row = (id, worth, divider) => `<div class="row" data-quest="${id}" data-worth="${worth}"><span class="mark"></span><span class="name">Quest ${id}</span>${divider ? '<span class="quest-divider"></span>' : ''}</div>`;
  const body = compliant
    ? `<section aria-labelledby="h-yes"><p class="group-label" id="h-yes">Worth it</p>${row('a', 'yes', true)}${row('b', 'yes', false)}</section>
       <section aria-labelledby="h-opt"><div class="section-break"></div><p class="group-label" id="h-opt">Optional</p>${row('c', 'optional', true)}${row('d', 'optional', false)}</section>`
    : `<section><p class="group-label">Worth it</p>${row('a', 'yes', true)}${row('b', 'yes', true)}</section>
       <p class="group-label">Optional</p>${row('c', 'optional', true)}${row('d', 'optional', false)}`;
  return `<!doctype html><html><head><style>
    html, body { margin: 0; background: #090D10; color: #E4E6E7; font: 16px Georgia, serif; }
    .group-label { margin: 0; padding: 8px 4px 4px; }
    .row { position: relative; display: flex; align-items: flex-start; gap: 10px; padding: 4px 4px 14px; }
    .mark { flex: none; width: 24px; height: 24px; background: #818D95; }
    .quest-divider { position: absolute; right: 0; bottom: 0; left: 0; height: 10px; background: #353E44; }
    .quest-divider::before { content: ''; position: absolute; top: 1.5px; left: calc(50% - 3.5px); width: 7px; height: 7px; background: #A6AEB3; }
    .section-break { position: relative; height: 10px; background: #353E44; }
    ${compliant ? '.quest-divider { height: 1px; background: #5D686F; } .quest-divider::before { display: none; }' : ''}
    ${layout === 'inset' ? '.quest-divider { left: 38px; right: 4px; }' : ''}
  </style></head><body><div class="overlay" data-frame="panel-next-up" data-mode="play"><div class="panel"><div class="body">${body}</div></div></div></body></html>`;
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

async function selfCheck(page, padding, print) {
  let failures = 0;
  let control = true;
  let flushed = 0;
  let readable = 0;
  let played = 0;
  let sectioned = 0;
  for (const scale of SCALES) {
    // The flush check: the 12 px margin must be reported, the flush frame must pass.
    await page.setViewportSize({ width: 364, height: 384 });
    const flush = [];
    for (const margin of [12, 0]) {
      await page.setContent(flushPage(scale, margin, padding));
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
    await page.setContent(selfCheckPage(scale));
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
    const m = await page.evaluate(measureInPage, { interactive: INTERACTIVE });
    const { reasons, knot: pixels } = selfCheckReasons(m, mask);
    print(`self-check s${scale} @boxes: ${reasons.length ? `FAIL ${reasons.join('; ')}` : `ok (knot box ${pixels} art px, centre box 0)`}`);
    if (reasons.length) failures++;

    // Readability: the three bad runs each reported for their own rule, the compliant one not at all.
    await page.setContent(typePage(scale));
    await settle(page);
    const type = await readabilityReasons(page);
    const of = (key) => type.reasons.filter((r) => r.startsWith(`text "${TYPE_RUNS[key]}" `));
    const typeProblems = [];
    if (!of('tiny').some((r) => r.includes('readability meta'))) typeProblems.push('the 9px run was not reported');
    if (!of('dim').some((r) => r.includes('contrast'))) typeProblems.push('the #535D64 run was not reported');
    if (!of('caps').some((r) => r.includes('caps'))) typeProblems.push('the small-caps run was not reported');
    if (of('good').length) typeProblems.push(`the compliant run was reported: ${named(of('good'))}`);
    if (type.runs !== 4) typeProblems.push(`${type.runs} of 4 runs read`);
    print(`self-check s${scale} @readability: ${typeProblems.length ? `FAIL ${typeProblems.join('; ')}` : `ok (${named(type.reasons)}; compliant run passes)`}`);
    if (typeProblems.length) failures++;
    readable++;

    // Play scrollbar: a visible bar on a play scroller reported, a hidden one passes and still scrolls.
    const bars = [];
    for (const hidden of [false, true]) {
      await page.setContent(scrollbarPage(hidden));
      await settle(page);
      const mode = await page.evaluate(() => document.querySelector('.overlay')?.getAttribute('data-mode') ?? null);
      const scrollers = await page.evaluate(scrollersInPage);
      bars.push({ scrollers, reasons: scrollbarReasons(mode, scrollers) });
    }
    const [shown, hiddenBar] = bars;
    const barProblems = [];
    if (shown.scrollers.length !== 1 || hiddenBar.scrollers.length !== 1) barProblems.push(`scrollers found: ${shown.scrollers.length} and ${hiddenBar.scrollers.length}, expected 1 each`);
    if (!shown.reasons.some((r) => r.startsWith('play scrollbar visible'))) barProblems.push('the visible play scrollbar was not reported');
    if (hiddenBar.reasons.length) barProblems.push(`the hidden play scrollbar was reported: ${named(hiddenBar.reasons)}`);
    print(`self-check s${scale} @scrollbar: ${barProblems.length ? `FAIL ${barProblems.join('; ')}` : `ok (${named(shown.reasons)}; hidden bar passes)`}`);
    if (barProblems.length) failures++;
    played++;

    // Sections: the flat layout reported for each section rule it breaks, the
    // full-width fine line reported as not inset, the inset layout passes.
    const pages = [];
    for (const layout of ['flat', 'full', 'inset']) {
      await page.setContent(sectionsPage(layout));
      await settle(page);
      const sections = await page.evaluate(sectionsInPage);
      pages.push({ sections, reasons: sections ? sectionReasons(sections) : [] });
    }
    const [flat, full, grouped] = pages;
    const sectionProblems = [];
    if (pages.some((p) => p.sections?.length !== 2)) sectionProblems.push(`sections found: ${pages.map((p) => p.sections?.length ?? 0).join(', ')}, expected 2 each`);
    for (const want of ['section heading missing yes', 'section heading missing optional', 'separator before heading', 'section break missing optional', 'section break not distinct']) {
      if (!flat.reasons.includes(want)) sectionProblems.push(`the flat layout was not reported: ${want}`);
    }
    if (full.reasons.length !== 1 || full.reasons[0] !== 'separator not inset') sectionProblems.push(`the full-width line was not reported as only separator not inset: ${named(full.reasons) || 'nothing'}`);
    if (grouped.reasons.length) sectionProblems.push(`the inset layout was reported: ${named(grouped.reasons)}`);
    print(`self-check s${scale} @sections: ${sectionProblems.length ? `FAIL ${sectionProblems.join('; ')}` : `ok (${named(flat.reasons)}; full-width line: ${named(full.reasons)}; inset layout passes)`}`);
    if (sectionProblems.length) failures++;
    sectioned++;
  }
  return { cases: SCALES.length, failures, control, flushed, readable, played, sectioned };
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
    if (port !== 1420) return port;
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

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const lines = [];
  const print = (line) => {
    lines.push(line);
    console.log(line);
  };
  // Playwright hides every scrollbar in headless mode; the app shows them, and
  // the play-scrollbar and scrollbar-as-content checks need them drawn.
  const browser = await chromium.launch({ channel: 'msedge', headless: true, ignoreDefaultArgs: ['--hide-scrollbars'] });
  let server = null;
  try {
    const context = await browser.newContext({ deviceScaleFactor: 1, viewport: { width: 364, height: 384 } });
    const page = await context.newPage();
    // The kit frame's own transparent padding, which alone may leave the window.
    const padding = artPadding(alphaMask(decodePng(new Uint8Array(fs.readFileSync(KIT_FRAME)), inflate)));
    if (args.selfCheck) {
      const r = await selfCheck(page, padding, print);
      console.log(summaryLine(r.cases, r.failures, r.control, r.flushed, r.readable, r.played, r.sectioned));
      return r.failures === 0 && r.control ? 0 : 1;
    }

    const vite = await loadVite(args.root);
    const port = await freePort();
    // The browser's benign "ResizeObserver loop" report, which the dev server
    // forwards from the page, says nothing about a case: keep it off the output.
    const logger = vite.createLogger('warn');
    const error = logger.error.bind(logger);
    logger.error = (message, options) => {
      if (!/ResizeObserver loop/.test(String(message))) error(message, options);
    };
    server = await vite.createServer({
      root: args.root,
      configFile: path.join(args.root, 'vite.config.ts'),
      customLogger: logger,
      clearScreen: false,
      server: { port, strictPort: true, host: '127.0.0.1', hmr: false },
    });
    await server.listen();
    const base = `http://127.0.0.1:${port}`;
    // Warm both pages, so a dependency optimisation reload lands before the matrix.
    for (const url of ['/?frame=panel-next-up&theme=exile', '/settings.html?theme=exile']) {
      await page.goto(base + url, { waitUntil: 'networkidle' });
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
    let played = 0;
    let sectioned = 0;
    const controlled = new Map();
    for (const c of declared) {
      const withControl = !controlled.has(c.scale);
      try {
        const r = await runCase(page, base, c, withControl, padding, print);
        failures += r.failures;
        flushed += r.flushed;
        readable += r.readable;
        played += r.played;
        sectioned += r.sectioned;
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

    if (args.screenshots) {
      fs.mkdirSync(args.screenshots, { recursive: true });
      for (const [name, url, width, height] of SCREENSHOTS) {
        await page.setViewportSize({ width, height });
        await page.goto(base + url, { waitUntil: 'load' });
        await page.waitForSelector(url.startsWith('/settings') ? '.window[data-state]:not([data-state="none"])' : '.overlay[data-frame]:not([data-frame="none"])', { timeout: 15000 });
        await settle(page);
        await page.screenshot({ path: path.join(args.screenshots, `${name}.png`), omitBackground: true });
      }
      console.log(`corner-safety: ${SCREENSHOTS.length} screenshots in ${args.screenshots}`);
    }

    console.log(summaryLine(declared.length, failures, control, flushed, readable, played, sectioned));
    return failures === 0 && ran === declared.length && control ? 0 : 1;
  } finally {
    await browser.close();
    if (server) await server.close();
  }
}

main().then(
  (code) => process.exit(code),
  (error) => {
    console.error(`corner-safety: ${error instanceof UsageError ? error.message : error?.stack || error}`);
    process.exit(2);
  },
);
