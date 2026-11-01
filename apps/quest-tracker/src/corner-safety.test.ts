import { describe, expect, it } from 'vitest';
import {
  ART_ALPHA,
  FLUSH_TOLERANCE,
  alphaMask,
  artPadding,
  artPixels,
  clipRect,
  clippedArt,
  containsRect,
  decodePng,
  findOverlaps,
  flushGaps,
  flushReasons,
  grow,
  MIN_CONTRAST,
  TYPE_FLOORS,
  contrastRatio,
  contrastReasons,
  parseColour,
  scrollbarReasons,
  summaryLine,
  typeReasons,
  worstContrast,
} from '../scripts/corner-safety-core.mjs';

// The pure core of `npm run corner-safety` (scripts/corner-safety-core.mjs),
// on the kit frame's real alpha. The runner feeds the same functions a
// browser screenshot of the rendered frame; here the mask is the source PNG
// at its own size (1189×1323, slice 224), decoded in Node with zlib.

const KIT_FRAME = '../../../docs/quest-tracker/design/steel-masonry-kit/assets/frame-neutral-steel.png';

type Inflate = (data: Uint8Array) => Uint8Array;

async function kitFrame() {
  const fs = (await import(/* @vite-ignore */ 'node:fs' as string)) as { readFileSync(path: URL): Uint8Array };
  const zlib = (await import(/* @vite-ignore */ 'node:zlib' as string)) as { inflateSync: Inflate };
  const bytes = fs.readFileSync(new URL(KIT_FRAME, import.meta.url));
  return decodePng(new Uint8Array(bytes), (data) => new Uint8Array(zlib.inflateSync(data)));
}

/** The bounding box of the mask's art pixels, right/bottom exclusive. */
function artBounds(mask: { width: number; height: number; art: Uint8Array }) {
  let left = mask.width, top = mask.height, right = 0, bottom = 0;
  for (let y = 0; y < mask.height; y++) {
    for (let x = 0; x < mask.width; x++) {
      if (!mask.art[y * mask.width + x]) continue;
      left = Math.min(left, x); top = Math.min(top, y);
      right = Math.max(right, x + 1); bottom = Math.max(bottom, y + 1);
    }
  }
  return [left, top, right, bottom];
}

describe('the corner-safety core', () => {
  it('corner_safety_decodes_the_kit_frame_alpha', async () => {
    const image = await kitFrame();
    expect([image.width, image.height]).toEqual([1189, 1323]);
    // Bounds measured independently with PIL at planning: any alpha at
    // (33,40)–(1157,1273), artwork (alpha >= 64) at (42,44)–(1150,1259).
    expect(artBounds(alphaMask(image, 1))).toEqual([33, 40, 1157, 1273]);
    expect(ART_ALPHA).toBe(64);
    expect(artBounds(alphaMask(image))).toEqual([42, 44, 1150, 1259]);
    // The centre is clear.
    expect(image.rgba[(661 * image.width + 594) * 4 + 3]).toBe(0);
  });

  it('corner_safety_reports_a_box_on_a_knot_and_not_one_in_the_centre', async () => {
    const mask = alphaMask(await kitFrame());
    // Positive control: a box on the bottom-left knot, every pixel artwork.
    const knot = { left: 60, top: 1170, right: 100, bottom: 1230 };
    // Negative control: a box in the clear centre.
    const centre = { left: 400, top: 500, right: 800, bottom: 800 };
    expect(artPixels(mask, knot)).toBe(40 * 60);
    expect(artPixels(mask, centre)).toBe(0);
    const overlaps = findOverlaps(mask, [
      { label: 'knot probe', rect: knot },
      { label: 'centre probe', rect: centre },
    ]);
    expect(overlaps.map((o) => o.label)).toEqual(['knot probe']);
    expect(overlaps[0].pixels).toBe(2400);

    // A box that would cross the knot but is clipped by its scroller to the
    // clear interior paints nothing over the art.
    const scroller = { left: 200, top: 200, right: 989, bottom: 1100 };
    expect(findOverlaps(mask, [{ label: 'row', rect: { left: 60, top: 1000, right: 900, bottom: 1230 }, clips: [scroller] }])).toEqual([]);
    // Unclipped, the same row does.
    expect(findOverlaps(mask, [{ label: 'row', rect: { left: 60, top: 1000, right: 900, bottom: 1230 } }])).toHaveLength(1);
  });

  it('corner_safety_counts_a_pixel_by_its_centre', () => {
    // A 4×4 mask with art in its right half.
    const mask = { width: 4, height: 4, art: new Uint8Array([0, 0, 1, 1, 0, 0, 1, 1, 0, 0, 1, 1, 0, 0, 1, 1]) };
    // Touching the art along an edge is not overlapping...
    expect(artPixels(mask, { left: 0, top: 0, right: 2, bottom: 4 })).toBe(0);
    // ...nor is a sliver short of a pixel centre...
    expect(artPixels(mask, { left: 0, top: 0, right: 2.4, bottom: 4 })).toBe(0);
    // ...but covering a centre is.
    expect(artPixels(mask, { left: 0, top: 0, right: 2.6, bottom: 4 })).toBe(4);
    // Outside the mask counts nothing rather than wrapping.
    expect(artPixels(mask, { left: -10, top: -10, right: 0, bottom: 10 })).toBe(0);
  });

  it('corner_safety_clips_and_grows_boxes', () => {
    const box = { left: 10, top: 10, right: 30, bottom: 30 };
    expect(clipRect(box, [{ left: 20, top: 0, right: 100, bottom: 100 }])).toEqual({ left: 20, top: 10, right: 30, bottom: 30 });
    expect(clipRect(box, [{ left: 40, top: 0, right: 100, bottom: 100 }])).toBeNull();
    expect(grow(box, 2)).toEqual({ left: 8, top: 8, right: 32, bottom: 32 });
    expect(containsRect({ left: 0, top: 0, right: 30, bottom: 30 }, box)).toBe(true);
    expect(containsRect({ left: 0, top: 0, right: 29, bottom: 30 }, box)).toBe(false);
    expect(summaryLine(3, 0, true, 3, 3, 1)).toBe(
      'corner-safety: 3 cases, 0 failures, control detected, flush checked on 3 cases, readability checked on 3 cases, play scrollbar checked on 1 cases',
    );
    expect(summaryLine(3, 1, false, 0, 0, 0)).toBe(
      'corner-safety: 3 cases, 1 failures, control not detected, flush checked on 0 cases, readability checked on 0 cases, play scrollbar checked on 0 cases',
    );
  });
});

// The flush check (owner, 2026-11-01): in Exile the frame's visible art meets
// every window edge, and no art pixel is cut off by the window; only the
// PNG's own transparent padding may fall outside it.
describe('the flush check', () => {
  /** A window-sized mask with art in a 6 px band just inside `inset` px from each edge. */
  function ring(width: number, height: number, inset: number) {
    const art = new Uint8Array(width * height);
    for (let y = inset; y < height - inset; y++) {
      for (let x = inset; x < width - inset; x++) {
        const band = x < inset + 6 || x >= width - inset - 6 || y < inset + 6 || y >= height - inset - 6;
        art[y * width + x] = band ? 1 : 0;
      }
    }
    return { width, height, art };
  }

  /** The kit frame drawn as the 48 px nine-slice on `rect` with these outsets (top, right, bottom, left). */
  function drawn(rect: { left: number; top: number; right: number; bottom: number }, [top, right, bottom, left]: number[]) {
    const all = (n: number) => ({ left: n, top: n, right: n, bottom: n });
    return { rect, outset: { left, top, right, bottom }, width: all(48), slice: all(224) };
  }

  it('corner_safety_reads_the_kit_frame_padding', async () => {
    // alpha >= 64 starts 42 / 44 source px in from the left and top and ends
    // 39 / 64 px short of the right and bottom (measured with PIL at planning).
    expect(artPadding(alphaMask(await kitFrame()))).toEqual({ left: 42, top: 44, right: 39, bottom: 64 });
  });

  it('corner_safety_reports_a_12px_margin_as_not_flush', () => {
    expect(FLUSH_TOLERANCE).toBe(2);
    // The frame as it was: art about 12 px in from every edge.
    const margin = ring(364, 384, 12);
    expect(flushGaps(margin)).toEqual([
      'left art 12px from the window edge',
      'top art 12px from the window edge',
      'right art 12px from the window edge',
      'bottom art 12px from the window edge',
    ]);
    // Flush, and within the 2 px allowance, passes.
    expect(flushGaps(ring(364, 384, 0))).toEqual([]);
    expect(flushGaps(ring(364, 384, 2))).toEqual([]);
    expect(flushGaps(ring(364, 384, 3))).toHaveLength(4);
    expect(flushGaps({ width: 4, height: 4, art: new Uint8Array(16) })).toEqual(['no art rendered']);
  });

  it('corner_safety_reports_art_the_window_cuts_off', async () => {
    const padding = artPadding(alphaMask(await kitFrame()));
    const viewport = { left: 0, top: 0, right: 364, bottom: 384 };
    const full = { left: 0, top: 0, right: 364, bottom: 384 };
    // Outsets a little under the padding at 48 px (9 / 9.43 / 8.36 / 13.71):
    // only transparent padding leaves the window.
    expect(clippedArt(drawn(full, [9, 8, 13.5, 8.5]), padding, viewport)).toEqual([]);
    // The 12 px margin's frame (8 px out from a box 12 px in) cuts nothing either...
    const inset = { left: 12, top: 12, right: 352, bottom: 372 };
    expect(clippedArt(drawn(inset, [8, 8, 8, 8]), padding, viewport)).toEqual([]);
    // ...but an outset past the padding cuts the art off.
    expect(clippedArt(drawn(full, [14, 8, 13.5, 8.5]), padding, viewport)).toEqual(['top art cut 4.6px by the window']);
    expect(clippedArt(drawn(full, [9, 12, 20, 8.5]), padding, viewport)).toEqual([
      'right art cut 3.6px by the window',
      'bottom art cut 6.3px by the window',
    ]);

    // Together: the margin frame is not flush, the flush frame passes.
    expect(flushReasons(ring(364, 384, 12), drawn(inset, [8, 8, 8, 8]), padding)).toHaveLength(4);
    expect(flushReasons(ring(364, 384, 0), drawn(full, [9, 8, 13.5, 8.5]), padding)).toEqual([]);
    expect(flushReasons(ring(364, 384, 0), null, padding)).toEqual(['no .material-art to measure']);
  });
});

// The readability check (owner, Live 2, 2026-11-02: text at the smaller
// scales was hard to read): a minimum rendered size per role, WCAG 2
// contrast of 4.5:1 against the worst pixel behind a run, and no faux small
// caps, thin weight or wide tracking.
describe('the readability check', () => {
  /** A run of `role` at `fontSize` CSS px under `zoom`, otherwise compliant. */
  function run(role: 'title' | 'body' | 'meta', fontSize: number, zoom: number, extra: Record<string, unknown> = {}) {
    return { role, fontSize, zoom, variantCaps: 'normal', weight: 400, letterSpacing: 0, capsFace: false, ...extra };
  }

  it('corner_safety_readability_floors_by_role_and_zoom', () => {
    expect(TYPE_FLOORS).toEqual({ title: 14, body: 13, meta: 12 });
    // 11 px metadata at scale 0.75 renders 8.25 px: under the 12 px floor.
    expect(typeReasons(run('meta', 11, 0.75))).toEqual(['readability meta 8.3px < 12']);
    // 12 px at scale 1 meets it, and so does a floor divided by the zoom.
    expect(typeReasons(run('meta', 12, 1))).toEqual([]);
    expect(typeReasons(run('meta', 12 / 0.75, 0.75))).toEqual([]);
    expect(typeReasons(run('body', 16, 0.75))).toEqual(['readability body 12px < 13']);
    expect(typeReasons(run('title', 19, 0.75))).toEqual([]);
    expect(typeReasons(run('title', 13, 1))).toEqual(['readability title 13px < 14']);
    // Text 200% passes on size.
    expect(typeReasons(run('meta', 22, 0.75))).toEqual([]);
  });

  it('corner_safety_readability_rejects_small_caps_thin_weight_and_wide_tracking', () => {
    expect(typeReasons(run('meta', 14, 1, { variantCaps: 'small-caps' }))).toEqual(['caps: font-variant-caps small-caps']);
    expect(typeReasons(run('meta', 14, 1, { variantCaps: 'all-small-caps' }))).toEqual(['caps: font-variant-caps all-small-caps']);
    // The caps face only at 13 px rendered and above.
    expect(typeReasons(run('meta', 12, 1, { capsFace: true }))).toEqual(['caps: the caps face at 12px < 13']);
    expect(typeReasons(run('meta', 13, 1, { capsFace: true }))).toEqual([]);
    expect(typeReasons(run('meta', 14, 1, { weight: 300 }))).toEqual(['weight 300 < 400']);
    // 0.08em tracking: too wide below 13 px, allowed above.
    expect(typeReasons(run('meta', 12, 1, { letterSpacing: 0.96 }))).toEqual(['tracking 0.08em > 0.05em']);
    expect(typeReasons(run('meta', 14, 1, { letterSpacing: 1.12 }))).toEqual([]);
    expect(typeReasons(run('meta', 14, 1, { letterSpacing: 2.1 }))).toEqual(['tracking 0.15em > 0.1em']);
  });

  it('corner_safety_contrast_is_wcag_4.5_against_the_worst_pixel', () => {
    expect(MIN_CONTRAST).toBe(4.5);
    // The kit text on the well passes; the disabled colour does not.
    expect(contrastRatio('#E4E6E7', '#090D10')).toBeCloseTo(15.58, 1);
    expect(contrastReasons(contrastRatio('#E4E6E7', '#090D10'))).toEqual([]);
    expect(contrastRatio('#535D64', '#090D10')).toBeLessThan(4.5);
    expect(contrastReasons(contrastRatio('#535D64', '#090D10'))).toEqual(['contrast 2.89 < 4.5']);
    expect(contrastRatio('#FFFFFF', '#000000')).toBeCloseTo(21, 5);
    // Computed colours as the browser reports them.
    expect(parseColour('rgb(228, 230, 231)')).toEqual({ r: 228, g: 230, b: 231, a: 1 });
    expect(parseColour('rgba(0, 0, 0, 0.5)')).toEqual({ r: 0, g: 0, b: 0, a: 0.5 });
    expect(parseColour('color(srgb 1 1 1 / 0.25)')).toEqual({ r: 255, g: 255, b: 255, a: 0.25 });
    // The worst pixel decides: one bright pixel under light text fails it.
    const fg = parseColour('#E4E6E7')!;
    const image = { width: 2, height: 1, rgba: new Uint8Array([9, 13, 16, 255, 200, 200, 200, 255]) };
    expect(worstContrast(fg, image, [{ left: 0, top: 0, right: 1, bottom: 1 }])).toBeCloseTo(15.58, 1);
    expect(worstContrast(fg, image, [{ left: 0, top: 0, right: 2, bottom: 1 }])!).toBeLessThan(MIN_CONTRAST);
    expect(worstContrast(fg, image, [])).toBeNull();
    // Half-transparent text is blended over the pixel first.
    expect(contrastRatio({ r: 228, g: 230, b: 231, a: 0.5 }, '#090D10')).toBeLessThan(contrastRatio('#E4E6E7', '#090D10'));
  });
});

// The play-scrollbar check (owner, Live 2: play is click-through): no
// scrollbar on an overflowing play scroller, which still scrolls; edit keeps one.
describe('the play-scrollbar check', () => {
  it('corner_safety_reports_a_play_scrollbar_and_a_missing_edit_one', () => {
    const shown = { label: 'div.body', thickness: 8, scrollable: true };
    const hidden = { label: 'div.body', thickness: 0, scrollable: true };
    expect(scrollbarReasons('play', [shown])).toEqual(['play scrollbar visible: div.body 8px']);
    expect(scrollbarReasons('play', [hidden])).toEqual([]);
    expect(scrollbarReasons('play', [{ ...hidden, scrollable: false }])).toEqual(['play not scrollable: div.body']);
    expect(scrollbarReasons('edit', [shown])).toEqual([]);
    expect(scrollbarReasons('edit', [hidden])).toEqual(['edit scrollbar missing: div.body']);
  });
});
