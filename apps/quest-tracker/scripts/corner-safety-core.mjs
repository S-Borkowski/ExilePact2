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
 * The runner's last line. `control` is whether the knot probe was reported
 * at every scale the run covered.
 *
 * @param {number} cases
 * @param {number} failures
 * @param {boolean} control
 */
export function summaryLine(cases, failures, control) {
  return `corner-safety: ${cases} cases, ${failures} failures, ${control ? 'control detected' : 'control not detected'}`;
}
