// The pure core of the corner-safety runner (`npm run corner-safety`):
// decode a PNG, turn its alpha into an artwork mask, and count the artwork
// pixels each content box covers. No DOM and no Node API, so the unit test
// (`src/corner-safety.test.ts`) runs the same code the runner does on the
// kit frame's real alpha, and the runner feeds it browser screenshots.
//
// Coordinates are CSS pixels of the viewport at device scale 1, which is
// what the runner screenshots: mask pixel (x, y) covers [x, x+1) × [y, y+1).
// A box covers a pixel when the pixel's centre lies strictly inside it, so a
// box that only touches the art along an edge does not count as overlapping.

/** A pixel at or above this alpha is artwork (the frame), below it is clear. */
export const ART_ALPHA = 64;

/**
 * @typedef {{ left: number, top: number, right: number, bottom: number }} Rect
 * @typedef {{ width: number, height: number, rgba: Uint8Array }} Image
 * @typedef {{ width: number, height: number, art: Uint8Array }} Mask
 * @typedef {{ label: string, rect: Rect, clips?: Rect[] }} ContentBox
 * @typedef {{ label: string, pixels: number, rect: Rect }} Overlap
 */

const SIGNATURE = [137, 80, 78, 71, 13, 10, 26, 10];

/** Bytes per pixel of each PNG colour type at bit depth 8. @type {Record<number, number>} */
const CHANNELS = { 0: 1, 2: 3, 3: 1, 4: 2, 6: 4 };

/** @param {Uint8Array} b @param {number} i */
function u32(b, i) {
  return ((b[i] << 24) | (b[i + 1] << 16) | (b[i + 2] << 8) | b[i + 3]) >>> 0;
}

/** @param {number} a @param {number} b @param {number} c */
function paeth(a, b, c) {
  const p = a + b - c;
  const pa = Math.abs(p - a);
  const pb = Math.abs(p - b);
  const pc = Math.abs(p - c);
  return pa <= pb && pa <= pc ? a : pb <= pc ? b : c;
}

/**
 * Decodes an 8-bit, non-interlaced PNG (greyscale, RGB, palette, grey+alpha
 * or RGBA) to RGBA. `inflate` is zlib's inflate (Node's `inflateSync`); it is
 * passed in so this module needs no Node API.
 *
 * @param {Uint8Array} bytes
 * @param {(data: Uint8Array) => Uint8Array} inflate
 * @returns {Image}
 */
export function decodePng(bytes, inflate) {
  if (bytes.length < 8 || SIGNATURE.some((v, i) => bytes[i] !== v)) throw new Error('not a PNG');
  let width = 0;
  let height = 0;
  let depth = 0;
  let colour = -1;
  let interlace = 0;
  /** @type {Uint8Array | null} */
  let palette = null;
  /** @type {Uint8Array | null} */
  let transparency = null;
  /** @type {Uint8Array[]} */
  const idat = [];
  let at = 8;
  while (at + 8 <= bytes.length) {
    const length = u32(bytes, at);
    const type = String.fromCharCode(bytes[at + 4], bytes[at + 5], bytes[at + 6], bytes[at + 7]);
    const data = bytes.subarray(at + 8, at + 8 + length);
    if (type === 'IHDR') {
      width = u32(data, 0);
      height = u32(data, 4);
      depth = data[8];
      colour = data[9];
      interlace = data[12];
    } else if (type === 'PLTE') palette = data;
    else if (type === 'tRNS') transparency = data;
    else if (type === 'IDAT') idat.push(data);
    else if (type === 'IEND') break;
    at += 12 + length;
  }
  if (depth !== 8) throw new Error(`PNG bit depth ${depth} is not supported (8 only)`);
  if (interlace !== 0) throw new Error('interlaced PNG is not supported');
  const channels = CHANNELS[colour];
  if (!channels) throw new Error(`PNG colour type ${colour} is not supported`);
  if (colour === 3 && !palette) throw new Error('palette PNG without PLTE');

  const joined = new Uint8Array(idat.reduce((n, c) => n + c.length, 0));
  let o = 0;
  for (const c of idat) {
    joined.set(c, o);
    o += c.length;
  }
  const raw = inflate(joined);
  const stride = width * channels;
  if (raw.length < height * (stride + 1)) throw new Error('PNG data is truncated');

  // Undo the per-row filters in place, into `rows` (no filter bytes).
  const rows = new Uint8Array(height * stride);
  for (let y = 0; y < height; y++) {
    const filter = raw[y * (stride + 1)];
    const src = y * (stride + 1) + 1;
    const dst = y * stride;
    for (let x = 0; x < stride; x++) {
      const a = x >= channels ? rows[dst + x - channels] : 0;
      const b = y > 0 ? rows[dst - stride + x] : 0;
      const c = x >= channels && y > 0 ? rows[dst - stride + x - channels] : 0;
      const v = raw[src + x];
      let out;
      switch (filter) {
        case 0: out = v; break;
        case 1: out = v + a; break;
        case 2: out = v + b; break;
        case 3: out = v + ((a + b) >> 1); break;
        case 4: out = v + paeth(a, b, c); break;
        default: throw new Error(`PNG filter ${filter} is not valid`);
      }
      rows[dst + x] = out & 0xff;
    }
  }

  const rgba = new Uint8Array(width * height * 4);
  for (let i = 0; i < width * height; i++) {
    const p = i * channels;
    const q = i * 4;
    if (colour === 6) {
      rgba[q] = rows[p]; rgba[q + 1] = rows[p + 1]; rgba[q + 2] = rows[p + 2]; rgba[q + 3] = rows[p + 3];
    } else if (colour === 2) {
      rgba[q] = rows[p]; rgba[q + 1] = rows[p + 1]; rgba[q + 2] = rows[p + 2]; rgba[q + 3] = 255;
    } else if (colour === 4) {
      rgba[q] = rgba[q + 1] = rgba[q + 2] = rows[p]; rgba[q + 3] = rows[p + 1];
    } else if (colour === 0) {
      rgba[q] = rgba[q + 1] = rgba[q + 2] = rows[p]; rgba[q + 3] = 255;
    } else {
      const k = rows[p];
      const pal = /** @type {Uint8Array} */ (palette);
      rgba[q] = pal[k * 3]; rgba[q + 1] = pal[k * 3 + 1]; rgba[q + 2] = pal[k * 3 + 2];
      rgba[q + 3] = transparency && k < transparency.length ? transparency[k] : 255;
    }
  }
  return { width, height, rgba };
}

/**
 * The artwork mask of a decoded image: 1 where alpha >= `threshold`.
 *
 * @param {Image} image
 * @param {number} [threshold]
 * @returns {Mask}
 */
export function alphaMask(image, threshold = ART_ALPHA) {
  const art = new Uint8Array(image.width * image.height);
  for (let i = 0; i < art.length; i++) art[i] = image.rgba[i * 4 + 3] >= threshold ? 1 : 0;
  return { width: image.width, height: image.height, art };
}

/** @param {Rect} a @param {Rect} b @returns {Rect | null} */
export function intersect(a, b) {
  const r = {
    left: Math.max(a.left, b.left),
    top: Math.max(a.top, b.top),
    right: Math.min(a.right, b.right),
    bottom: Math.min(a.bottom, b.bottom),
  };
  return r.right > r.left && r.bottom > r.top ? r : null;
}

/**
 * A box clipped by every clip rectangle in turn (a scroller's padding box,
 * the viewport); null when nothing of it is left to paint.
 *
 * @param {Rect} rect
 * @param {Rect[]} [clips]
 * @returns {Rect | null}
 */
export function clipRect(rect, clips = []) {
  /** @type {Rect | null} */
  let r = rect;
  for (const c of clips) {
    if (!r) break;
    r = intersect(r, c);
  }
  return r;
}

/** @param {Rect} rect @param {number} by @returns {Rect} */
export function grow(rect, by) {
  return { left: rect.left - by, top: rect.top - by, right: rect.right + by, bottom: rect.bottom + by };
}

/**
 * Whether `inner` lies inside `outer`, within `tolerance` px on each side.
 *
 * @param {Rect} outer
 * @param {Rect} inner
 * @param {number} [tolerance]
 */
export function containsRect(outer, inner, tolerance = 0.5) {
  return (
    inner.left >= outer.left - tolerance &&
    inner.top >= outer.top - tolerance &&
    inner.right <= outer.right + tolerance &&
    inner.bottom <= outer.bottom + tolerance
  );
}

/**
 * How many artwork pixels of `mask` have their centre inside `rect`.
 *
 * @param {Mask} mask
 * @param {Rect} rect
 */
export function artPixels(mask, rect) {
  // Pixel x is inside when x + 0.5 lies in (left, right).
  const x0 = Math.max(0, Math.floor(rect.left + 0.5));
  const x1 = Math.min(mask.width - 1, Math.ceil(rect.right - 0.5) - 1);
  const y0 = Math.max(0, Math.floor(rect.top + 0.5));
  const y1 = Math.min(mask.height - 1, Math.ceil(rect.bottom - 0.5) - 1);
  let n = 0;
  for (let y = y0; y <= y1; y++) {
    const row = y * mask.width;
    for (let x = x0; x <= x1; x++) n += mask.art[row + x];
  }
  return n;
}

/**
 * Every content box that paints over artwork, after its clips: the
 * corner-safety check proper.
 *
 * @param {Mask} mask
 * @param {ContentBox[]} boxes
 * @returns {Overlap[]}
 */
export function findOverlaps(mask, boxes) {
  /** @type {Overlap[]} */
  const out = [];
  for (const box of boxes) {
    const painted = clipRect(box.rect, box.clips);
    if (!painted) continue;
    const pixels = artPixels(mask, painted);
    if (pixels > 0) out.push({ label: box.label, pixels, rect: painted });
  }
  return out;
}

/**
 * The bounding box of a mask's art pixels, right and bottom exclusive; null
 * when it has none.
 *
 * @param {Mask} mask
 * @returns {Rect | null}
 */
export function artBounds(mask) {
  let left = mask.width;
  let top = mask.height;
  let right = 0;
  let bottom = 0;
  for (let y = 0; y < mask.height; y++) {
    const row = y * mask.width;
    for (let x = 0; x < mask.width; x++) {
      if (!mask.art[row + x]) continue;
      if (x < left) left = x;
      if (x + 1 > right) right = x + 1;
      if (y < top) top = y;
      bottom = y + 1;
    }
  }
  return right > left ? { left, top, right, bottom } : null;
}

/**
 * An image's own transparent padding: how many source px lie between each
 * of its edges and its art (alpha >= 64).
 *
 * @param {Mask} mask
 * @returns {{ left: number, top: number, right: number, bottom: number }}
 */
export function artPadding(mask) {
  const b = artBounds(mask);
  if (!b) throw new Error('the image has no art');
  return { left: b.left, top: b.top, right: mask.width - b.right, bottom: mask.height - b.bottom };
}

/** How near (css px) the rendered art must come to each window edge to count as flush. */
export const FLUSH_TOLERANCE = 2;

const SIDES = /** @type {const} */ (['left', 'top', 'right', 'bottom']);
/** @param {number} n */
const round1 = (n) => Math.round(n * 10) / 10;

/**
 * The flush check's first half, on the rendered art alone: each window edge
 * the art stays more than `tolerance` px away from. `mask` is a screenshot
 * of the whole viewport, so its size is the window's.
 *
 * @param {Mask} mask
 * @param {number} [tolerance]
 * @returns {string[]}
 */
export function flushGaps(mask, tolerance = FLUSH_TOLERANCE) {
  const b = artBounds(mask);
  if (!b) return ['no art rendered'];
  const gaps = { left: b.left, top: b.top, right: mask.width - b.right, bottom: mask.height - b.bottom };
  return SIDES.filter((s) => gaps[s] > tolerance).map((s) => `${s} art ${round1(gaps[s])}px from the window edge`);
}

/**
 * @typedef {{ left: number, top: number, right: number, bottom: number }} Sides
 * @typedef {{ rect: Rect, outset: Sides, width: Sides, slice: Sides }} DrawnFrame
 *   A nine-slice frame as the page draws it, in viewport px: the element's
 *   box, its border-image outset and width per side, and its slice per side
 *   in source px.
 */

/**
 * The flush check's second half: each side whose art (alpha >= 64) the
 * window cuts off. Only the image's own transparent padding may fall
 * outside the window. Computed from the frame's geometry and the source
 * image's padding, since art outside the window is not in any screenshot.
 *
 * @param {DrawnFrame} frame
 * @param {Sides} padding  source px, from `artPadding`
 * @param {Rect} viewport
 * @param {number} [tolerance]
 * @returns {string[]}
 */
export function clippedArt(frame, padding, viewport, tolerance = 0.5) {
  /** @param {keyof Sides} s */
  const scale = (s) => (frame.slice[s] > 0 ? frame.width[s] / frame.slice[s] : 0);
  const art = {
    left: frame.rect.left - frame.outset.left + padding.left * scale('left'),
    top: frame.rect.top - frame.outset.top + padding.top * scale('top'),
    right: frame.rect.right + frame.outset.right - padding.right * scale('right'),
    bottom: frame.rect.bottom + frame.outset.bottom - padding.bottom * scale('bottom'),
  };
  const cut = {
    left: viewport.left - art.left,
    top: viewport.top - art.top,
    right: art.right - viewport.right,
    bottom: art.bottom - viewport.bottom,
  };
  return SIDES.filter((s) => cut[s] > tolerance).map((s) => `${s} art cut ${round1(cut[s])}px by the window`);
}

/**
 * Every reason the frame is not flush with the window: art short of an
 * edge, or art cut off by one.
 *
 * @param {Mask} mask
 * @param {DrawnFrame | null} frame
 * @param {Sides} padding
 * @returns {string[]}
 */
export function flushReasons(mask, frame, padding) {
  const reasons = flushGaps(mask);
  if (!frame) reasons.push('no .material-art to measure');
  else reasons.push(...clippedArt(frame, padding, { left: 0, top: 0, right: mask.width, bottom: mask.height }));
  return reasons;
}

// ---------------------------------------------------------------- readability
// Owner, Live 2 (2026-11-02): text at the smaller scales was hard to read.
// Every text run is held to a minimum rendered size for its role, a WCAG 2
// contrast of 4.5:1 against the worst pixel behind it, and no faux small
// caps, thin weight or wide tracking at small sizes.

/** The minimum rendered size (window px) of each text role. */
export const TYPE_FLOORS = /** @type {const} */ ({ title: 14, body: 13, meta: 12 });
/** The minimum contrast of a text run against every pixel behind it (WCAG 2 AA, normal text). */
export const MIN_CONTRAST = 4.5;
/** Below this rendered size the caps face (Cinzel) is not used and tracking stays tight. */
export const SMALL_TEXT = 13;

/**
 * @typedef {{ r: number, g: number, b: number, a: number }} Colour  0-255 channels, alpha 0-1
 * @typedef {'title' | 'body' | 'meta'} Role
 * @typedef {{
 *   role: Role, fontSize: number, zoom: number, variantCaps: string, weight: number,
 *   letterSpacing: number, capsFace: boolean,
 * }} TypeRun  computed values in unzoomed CSS px; `zoom` turns them into window px
 */

/**
 * A computed CSS colour (`rgb()`, `rgba()`, `#hex`, or `color(srgb ...)`)
 * as 0-255 channels and an alpha; null for anything else.
 *
 * @param {string} css
 * @returns {Colour | null}
 */
export function parseColour(css) {
  const text = String(css).trim().toLowerCase();
  if (text === 'transparent') return { r: 0, g: 0, b: 0, a: 0 };
  const hex = text.match(/^#([0-9a-f]{3,8})$/)?.[1];
  if (hex && [3, 4, 6, 8].includes(hex.length)) {
    const full = hex.length <= 4 ? [...hex].map((c) => c + c).join('') : hex;
    const n = (/** @type {number} */ i) => parseInt(full.slice(i, i + 2), 16);
    return { r: n(0), g: n(2), b: n(4), a: full.length === 8 ? n(6) / 255 : 1 };
  }
  const srgb = text.match(/^color\(srgb\s+([\d.e+-]+)\s+([\d.e+-]+)\s+([\d.e+-]+)(?:\s*\/\s*([\d.e+-]+)(%?))?\s*\)$/);
  if (srgb) {
    const c = (/** @type {string} */ v) => Math.round(Math.min(1, Math.max(0, parseFloat(v))) * 255);
    const a = srgb[4] === undefined ? 1 : parseFloat(srgb[4]) / (srgb[5] ? 100 : 1);
    return { r: c(srgb[1]), g: c(srgb[2]), b: c(srgb[3]), a };
  }
  const rgb = text.match(/^rgba?\(([^)]*)\)$/);
  if (rgb) {
    const parts = rgb[1].split(/[\s,/]+/).filter(Boolean);
    if (parts.length < 3) return null;
    const c = (/** @type {string} */ v) => Math.round(v.endsWith('%') ? (parseFloat(v) * 255) / 100 : parseFloat(v));
    const a = parts[3] === undefined ? 1 : parts[3].endsWith('%') ? parseFloat(parts[3]) / 100 : parseFloat(parts[3]);
    return { r: c(parts[0]), g: c(parts[1]), b: c(parts[2]), a };
  }
  return null;
}

/** WCAG 2 relative luminance of an opaque colour. @param {Colour} c */
export function luminance(c) {
  const lin = (/** @type {number} */ v) => {
    const s = v / 255;
    return s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * lin(c.r) + 0.7152 * lin(c.g) + 0.0722 * lin(c.b);
}

/**
 * The WCAG 2 contrast ratio of two colours, `fg` laid over `bg` by its alpha
 * (the background is taken as opaque, as a screenshot pixel is).
 *
 * @param {Colour | string} fg
 * @param {Colour | string} bg
 */
export function contrastRatio(fg, bg) {
  const b = typeof bg === 'string' ? parseColour(bg) : bg;
  const f = typeof fg === 'string' ? parseColour(fg) : fg;
  if (!f || !b) throw new Error(`not a colour: ${!f ? fg : bg}`);
  const a = f.a ?? 1;
  const over = { r: f.r * a + b.r * (1 - a), g: f.g * a + b.g * (1 - a), b: f.b * a + b.b * (1 - a), a: 1 };
  const l1 = luminance(over);
  const l2 = luminance(b);
  return (Math.max(l1, l2) + 0.05) / (Math.min(l1, l2) + 0.05);
}

/**
 * The lowest contrast of `fg` against the screenshot pixels whose centre
 * lies inside each of `rects` (viewport px); null when no pixel is covered.
 *
 * @param {Colour} fg
 * @param {Image} image  the case's screenshot with every text run made transparent
 * @param {Rect[]} rects
 * @returns {number | null}
 */
export function worstContrast(fg, image, rects) {
  let worst = null;
  for (const rect of rects) {
    const x0 = Math.max(0, Math.floor(rect.left + 0.5));
    const x1 = Math.min(image.width - 1, Math.ceil(rect.right - 0.5) - 1);
    const y0 = Math.max(0, Math.floor(rect.top + 0.5));
    const y1 = Math.min(image.height - 1, Math.ceil(rect.bottom - 0.5) - 1);
    for (let y = y0; y <= y1; y++) {
      for (let x = x0; x <= x1; x++) {
        const i = (y * image.width + x) * 4;
        const ratio = contrastRatio(fg, { r: image.rgba[i], g: image.rgba[i + 1], b: image.rgba[i + 2], a: 1 });
        if (worst === null || ratio < worst) worst = ratio;
      }
    }
  }
  return worst;
}

/** The contrast rule on a measured ratio. @param {number | null} ratio */
export function contrastReasons(ratio) {
  return ratio !== null && ratio < MIN_CONTRAST ? [`contrast ${Math.floor(ratio * 100) / 100} < ${MIN_CONTRAST}`] : [];
}

/**
 * The size, caps, weight and tracking rules on one text run.
 *
 * @param {TypeRun} run
 * @returns {string[]}
 */
export function typeReasons(run) {
  const reasons = [];
  const rendered = run.fontSize * run.zoom;
  const floor = TYPE_FLOORS[run.role];
  // A hair under the floor is rounding in the zoom, not a smaller size.
  if (rendered < floor - 0.05) reasons.push(`readability ${run.role} ${round1(rendered)}px < ${floor}`);
  if (run.variantCaps && run.variantCaps !== 'normal') reasons.push(`caps: font-variant-caps ${run.variantCaps}`);
  if (run.capsFace && rendered < SMALL_TEXT - 0.05) reasons.push(`caps: the caps face at ${round1(rendered)}px < ${SMALL_TEXT}`);
  if (run.weight < 400) reasons.push(`weight ${run.weight} < 400`);
  const tracking = run.fontSize > 0 ? run.letterSpacing / run.fontSize : 0;
  const most = rendered < SMALL_TEXT - 0.05 ? 0.05 : 0.1;
  if (tracking > most + 0.001) reasons.push(`tracking ${Math.round(tracking * 1000) / 1000}em > ${most}em`);
  return reasons;
}

// ------------------------------------------------------------ play scrollbar
// Owner, Live 2: play is click-through, so a scrollbar there cannot be used.
// In play every overflowing scroller shows none and still scrolls (a data
// update or a restored position moves it); edit keeps the kit's scrollbar.

/**
 * @typedef {{ label: string, thickness: number, scrollable: boolean }} Scroller
 *   an overflowing scroller: its scrollbar's thickness (CSS px) and whether
 *   setting `scrollTop` moved it
 */

/**
 * @param {string | null} mode  the overlay's `data-mode`
 * @param {Scroller[]} scrollers
 * @returns {string[]}
 */
export function scrollbarReasons(mode, scrollers) {
  const reasons = [];
  for (const s of scrollers) {
    if (mode === 'play') {
      if (s.thickness > 0.5) reasons.push(`play scrollbar visible: ${s.label} ${round1(s.thickness)}px`);
      if (!s.scrollable) reasons.push(`play not scrollable: ${s.label}`);
    } else if (mode === 'edit' && s.thickness <= 0.5) {
      reasons.push(`edit scrollbar missing: ${s.label}`);
    }
  }
  return reasons;
}

/**
 * The runner's last line. `control` is whether the knot probe was reported
 * at every scale the run covered; `flushed` how many cases the flush check
 * ran on, `readable` how many had their text runs checked, and `played` how
 * many play cases had an overflowing scroller checked.
 *
 * @param {number} cases
 * @param {number} failures
 * @param {boolean} control
 * @param {number} flushed
 * @param {number} readable
 * @param {number} played
 */
export function summaryLine(cases, failures, control, flushed, readable, played) {
  return `corner-safety: ${cases} cases, ${failures} failures, ${control ? 'control detected' : 'control not detected'}, flush checked on ${flushed} cases, readability checked on ${readable} cases, play scrollbar checked on ${played} cases`;
}
