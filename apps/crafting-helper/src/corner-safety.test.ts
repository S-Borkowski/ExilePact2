import { describe, expect, it } from 'vitest';
import {
  ART_ALPHA,
  FLUSH_TOLERANCE,
  MIN_CONTRAST,
  MIN_TEXT,
  SMALL_TEXT,
  alphaMask,
  artPadding,
  artPixels,
  clipRect,
  clippedArt,
  containsRect,
  contrastRatio,
  contrastReasons,
  decodePng,
  findOverlaps,
  flushGaps,
  flushReasons,
  grow,
  legacyLine,
  parseColour,
  pixelDiff,
  summaryLine,
  typeReasons,
  worstContrast,
} from '../scripts/corner-safety-core.mjs';

// The pure core of `npm run corner-safety` (scripts/corner-safety-core.mjs),
// adapted from the quest tracker's, on the kit frame's real alpha. The runner
// feeds the same functions a browser screenshot of the rendered frame; here
// the mask is the source PNG at its own size (1189 x 1323, slice 224),
// decoded in Node with zlib (no new dependency).

const KIT_FRAME = '../../../docs/quest-tracker/design/steel-masonry-kit/assets/frame-neutral-steel.png';

type Inflate = (data: Uint8Array) => Uint8Array;

async function kitFrame() {
  const fs = (await import(/* @vite-ignore */ 'node:fs' as string)) as { readFileSync(path: URL): Uint8Array };
  const zlib = (await import(/* @vite-ignore */ 'node:zlib' as string)) as { inflateSync: Inflate };
  const bytes = fs.readFileSync(new URL(KIT_FRAME, import.meta.url));
  return decodePng(new Uint8Array(bytes), (data) => new Uint8Array(zlib.inflateSync(data)));
}

describe('the corner-safety core', () => {
  it('corner_safety_decodes_the_kit_frame_alpha', async () => {
    const image = await kitFrame();
    expect([image.width, image.height]).toEqual([1189, 1323]);
    expect(ART_ALPHA).toBe(64);
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

    // A row clipped by its scroller to the clear interior paints nothing over the art...
    const scroller = { left: 200, top: 200, right: 989, bottom: 1100 };
    expect(findOverlaps(mask, [{ label: 'row', rect: { left: 60, top: 1000, right: 900, bottom: 1230 }, clips: [scroller] }])).toEqual([]);
    // ...unclipped, the same row does.
    expect(findOverlaps(mask, [{ label: 'row', rect: { left: 60, top: 1000, right: 900, bottom: 1230 } }])).toHaveLength(1);
  });

  it('corner_safety_counts_a_pixel_by_its_centre_and_clips_boxes', () => {
    const mask = { width: 4, height: 4, art: new Uint8Array([0, 0, 1, 1, 0, 0, 1, 1, 0, 0, 1, 1, 0, 0, 1, 1]) };
    expect(artPixels(mask, { left: 0, top: 0, right: 2, bottom: 4 })).toBe(0);
    expect(artPixels(mask, { left: 0, top: 0, right: 2.4, bottom: 4 })).toBe(0);
    expect(artPixels(mask, { left: 0, top: 0, right: 2.6, bottom: 4 })).toBe(4);
    expect(artPixels(mask, { left: -10, top: -10, right: 0, bottom: 10 })).toBe(0);
    const box = { left: 10, top: 10, right: 30, bottom: 30 };
    expect(clipRect(box, [{ left: 20, top: 0, right: 100, bottom: 100 }])).toEqual({ left: 20, top: 10, right: 30, bottom: 30 });
    expect(clipRect(box, [{ left: 40, top: 0, right: 100, bottom: 100 }])).toBeNull();
    expect(grow(box, 2)).toEqual({ left: 8, top: 8, right: 32, bottom: 32 });
    expect(containsRect({ left: 0, top: 0, right: 30, bottom: 30 }, box)).toBe(true);
    expect(containsRect({ left: 0, top: 0, right: 29, bottom: 30 }, box)).toBe(false);
  });

  it('corner_safety_summary_lines', () => {
    expect(summaryLine(3, 0, true, 3, 3)).toBe(
      'corner-safety: 3 cases, 0 failures, control detected, flush checked on 3 cases, readability checked on 3 cases',
    );
    expect(summaryLine(3, 1, false, 0, 0)).toBe(
      'corner-safety: 3 cases, 1 failures, control not detected, flush checked on 0 cases, readability checked on 0 cases',
    );
    expect(legacyLine(26, 0)).toBe('legacy-pixels: 26 cases, 0 differ');
  });
});

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

  function drawn(rect: { left: number; top: number; right: number; bottom: number }, [top, right, bottom, left]: number[]) {
    const all = (n: number) => ({ left: n, top: n, right: n, bottom: n });
    return { rect, outset: { left, top, right, bottom }, width: all(48), slice: all(224) };
  }

  it('corner_safety_reads_the_kit_frame_padding', async () => {
    expect(artPadding(alphaMask(await kitFrame()))).toEqual({ left: 42, top: 44, right: 39, bottom: 64 });
  });

  it('corner_safety_reports_a_12px_margin_as_not_flush_and_a_flush_mask_passes', () => {
    expect(FLUSH_TOLERANCE).toBe(2);
    expect(flushGaps(ring(960, 680, 12))).toEqual([
      'left art 12px from the window edge',
      'top art 12px from the window edge',
      'right art 12px from the window edge',
      'bottom art 12px from the window edge',
    ]);
    expect(flushGaps(ring(960, 680, 0))).toEqual([]);
    expect(flushGaps(ring(960, 680, 2))).toEqual([]);
    expect(flushGaps({ width: 4, height: 4, art: new Uint8Array(16) })).toEqual(['no art rendered']);
  });

  it('corner_safety_reports_art_the_window_cuts_off', async () => {
    const padding = artPadding(alphaMask(await kitFrame()));
    const viewport = { left: 0, top: 0, right: 440, bottom: 720 };
    const full = { left: 0, top: 0, right: 440, bottom: 720 };
    // The quest tracker's flush outsets cut only transparent padding.
    expect(clippedArt(drawn(full, [9, 8, 13.5, 8.5]), padding, viewport)).toEqual([]);
    expect(clippedArt(drawn(full, [14, 8, 13.5, 8.5]), padding, viewport)).toEqual(['top art cut 4.6px by the window']);
    expect(flushReasons(ring(440, 720, 0), drawn(full, [9, 8, 13.5, 8.5]), padding)).toEqual([]);
    expect(flushReasons(ring(440, 720, 0), null, padding)).toEqual(['no .material-art to measure']);
  });
});

describe('the readability check', () => {
  function run(fontSize: number, extra: Record<string, unknown> = {}) {
    return { fontSize, zoom: 1, variantCaps: 'normal', weight: 400, letterSpacing: 0, capsFace: false, ...extra };
  }

  it('corner_safety_readability_fails_an_11px_run', () => {
    expect(MIN_TEXT).toBe(12);
    expect(SMALL_TEXT).toBe(13);
    expect(typeReasons(run(11))).toEqual(['readability 11px < 12']);
    expect(typeReasons(run(12))).toEqual([]);
    // Text 200% passes on size.
    expect(typeReasons(run(24))).toEqual([]);
  });

  it('corner_safety_readability_rejects_small_caps_thin_weight_wide_tracking_and_small_cinzel', () => {
    expect(typeReasons(run(14, { variantCaps: 'small-caps' }))).toEqual(['caps: font-variant-caps small-caps']);
    expect(typeReasons(run(12, { capsFace: true }))).toEqual(['caps: the caps face at 12px < 13']);
    expect(typeReasons(run(13, { capsFace: true }))).toEqual([]);
    expect(typeReasons(run(14, { weight: 300 }))).toEqual(['weight 300 < 400']);
    expect(typeReasons(run(12, { letterSpacing: 0.96 }))).toEqual(['tracking 0.08em > 0.05em']);
    expect(typeReasons(run(14, { letterSpacing: 1.12 }))).toEqual([]);
    expect(typeReasons(run(14, { letterSpacing: 2.1 }))).toEqual(['tracking 0.15em > 0.1em']);
  });

  it('corner_safety_contrast_below_4.5_fails', () => {
    expect(MIN_CONTRAST).toBe(4.5);
    expect(contrastReasons(contrastRatio('#E4E6E7', '#090D10'))).toEqual([]);
    expect(contrastReasons(contrastRatio('#535D64', '#090D10'))).toEqual(['contrast 2.89 < 4.5']);
    expect(parseColour('rgb(228, 230, 231)')).toEqual({ r: 228, g: 230, b: 231, a: 1 });
    expect(parseColour('color(srgb 1 1 1 / 0.25)')).toEqual({ r: 255, g: 255, b: 255, a: 0.25 });
    const fg = parseColour('#E4E6E7')!;
    const image = { width: 2, height: 1, rgba: new Uint8Array([9, 13, 16, 255, 200, 200, 200, 255]) };
    expect(worstContrast(fg, image, [{ left: 0, top: 0, right: 1, bottom: 1 }])!).toBeGreaterThan(MIN_CONTRAST);
    expect(worstContrast(fg, image, [{ left: 0, top: 0, right: 2, bottom: 1 }])!).toBeLessThan(MIN_CONTRAST);
    expect(worstContrast(fg, image, [])).toBeNull();
  });
});

describe('the legacy pixel comparison', () => {
  it('corner_safety_pixel_diff_counts_each_differing_pixel', () => {
    const a = { width: 2, height: 2, rgba: new Uint8Array(16).fill(10) };
    const same = { width: 2, height: 2, rgba: new Uint8Array(16).fill(10) };
    expect(pixelDiff(a, same)).toBe(0);
    const one = { width: 2, height: 2, rgba: new Uint8Array(16).fill(10) };
    one.rgba[5] = 11;
    expect(pixelDiff(a, one)).toBe(1);
    // A size mismatch differs in every pixel of the larger image.
    expect(pixelDiff(a, { width: 3, height: 2, rgba: new Uint8Array(24).fill(10) })).toBe(6);
  });
});
