// The settings sliders apply while they are dragged (owner, after Live 3:
// "opacity slider should be responsive too", scale "not only when picker is
// unheld"). Two pieces, kept out of the component so the tests can drive them:
//
// - `frameSender` coalesces a stream of values into at most one send per
//   animation frame, always the newest, and `flush` sends the last one at
//   once (the release).
// - `travelValue` maps the pointer's travel since the press to a value. A
//   fixed scale is the webview zoom, so every update rescales the page under
//   the pointer: a range's own mapping (pointer position over the track in
//   CSS pixels) would chase its own zoom. Travel and the track's width are
//   measured in screen pixels instead (CSS pixels times `devicePixelRatio`,
//   which carries the webview zoom), the width once, at the press, so the
//   zoom changing under the drag does not move the value.

/** A value stream sent at most once per animation frame. */
export interface FrameSender<T> {
  /** Queue `value`; it goes out on the next animation frame, unless a newer one replaces it first. */
  push(value: T): void;
  /** Send the queued value now (or `value`, when given), unless this gesture already sent it; ends the gesture. */
  flush(value?: T): void;
  /** Drop a queued value without sending it. */
  cancel(): void;
}

/**
 * `send` behind a once-per-frame gate: never two sends in one frame, and
 * within one gesture (up to a `flush`) never the same value twice in a row.
 */
export function frameSender<T>(send: (value: T) => void): FrameSender<T> {
  let frame: number | null = null;
  let pending: { value: T } | null = null;
  let last: { value: T } | null = null;

  const emit = (value: T) => {
    if (last && Object.is(last.value, value)) return;
    last = { value };
    send(value);
  };
  const stop = () => {
    if (frame !== null) cancelAnimationFrame(frame);
    frame = null;
  };

  return {
    push(value) {
      pending = { value };
      if (frame !== null) return;
      frame = requestAnimationFrame(() => {
        frame = null;
        const next = pending;
        pending = null;
        if (next) emit(next.value);
      });
    },
    flush(value) {
      stop();
      const next = value === undefined ? pending : { value };
      pending = null;
      if (next) emit(next.value);
      // The next gesture starts fresh: the setting may have changed another way since.
      last = null;
    },
    cancel() {
      stop();
      pending = null;
    },
  };
}

export interface Range {
  min: number;
  max: number;
  step: number;
}

/** `value` brought into the range and onto its step grid (to hundredths, so 0.05 steps stay exact). */
export function snap(value: number, { min, max, step }: Range): number {
  const clamped = Math.min(max, Math.max(min, value));
  const stepped = min + Math.round((clamped - min) / step) * step;
  return Math.round(Math.min(max, stepped) * 100) / 100;
}

/**
 * The value a drag has reached: `start` plus the travel since the press over
 * the track's usable width, both in screen pixels, across the range's span.
 */
export function travelValue(start: number, travelPx: number, widthPx: number, range: Range): number {
  if (!(widthPx > 0)) return snap(start, range);
  return snap(start + (travelPx / widthPx) * (range.max - range.min), range);
}

/** The thumb's width in CSS pixels (`styled.css`, `.slider::-webkit-slider-thumb`). */
export const THUMB_PX = 8;

/** The screen-pixel scale of one CSS pixel right now (1 where the webview reports none). */
export function screenScale(): number {
  const ratio = typeof window === 'undefined' ? 1 : window.devicePixelRatio;
  return ratio > 0 ? ratio : 1;
}

/**
 * Where a press on a range starts its drag: a press on the thumb keeps the
 * current value (no jump under the pointer), a press on the track starts
 * from the value at that point, as a range does.
 */
export function pressValue(current: number, clientX: number, rect: { left: number; width: number }, range: Range): number {
  const usable = rect.width - THUMB_PX;
  if (!(usable > 0)) return current;
  const thumbCentre = rect.left + THUMB_PX / 2 + ((current - range.min) / (range.max - range.min)) * usable;
  if (Math.abs(clientX - thumbCentre) <= THUMB_PX) return current;
  return snap(range.min + ((clientX - rect.left - THUMB_PX / 2) / usable) * (range.max - range.min), range);
}
