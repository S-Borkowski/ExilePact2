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
  summaryLine,
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
    expect(summaryLine(3, 0, true, 3)).toBe('corner-safety: 3 cases, 0 failures, control detected, flush checked on 3 cases');
    expect(summaryLine(3, 1, false, 0)).toBe('corner-safety: 3 cases, 1 failures, control not detected, flush checked on 0 cases');
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
