<script lang="ts">
  // The Exile frame: the steel/masonry kit's neutral steel PNG, unchanged, as
  // a nine-slice border (224 source px per side, 48 px corners, the centre
  // left empty). Decorative only: hidden from assistive technology, never a
  // target. Its visible art meets the window's edge (owner, 2026-11-01): the
  // overlay and the ordinary Settings window both draw it over the whole
  // window, and each side's outset carries the PNG's own transparent padding
  // past the edge. That padding is 42 / 44 / 39 / 64 source px (left, top,
  // right, bottom), 9 / 9.4 / 8.4 / 13.7 px at 48 px; each outset stays a
  // little under it, so no art pixel is cut. It keeps one size on screen at
  // every scale, so only the content zooms (owner, 2026-11-01).
  import frame from '../../theme/materials/raster/frame-neutral-steel.png';
</script>

<div class="material-art" aria-hidden="true" style:--frame-image={`url("${frame}")`}></div>

<style>
  .material-art {
    --frame-zoom: var(--page-zoom, var(--preview-scale, 1));
    position: absolute;
    z-index: 1;
    inset: 0;
    pointer-events: none;
    border: 0 solid transparent;
    border-image-source: var(--frame-image);
    border-image-slice: 224;
    border-image-width: calc(48px / var(--frame-zoom));
    border-image-repeat: stretch;
    /* Top, right, bottom, left: just under the PNG's padding at 48 px. */
    border-image-outset: calc(9px / var(--frame-zoom)) calc(8px / var(--frame-zoom)) calc(13.5px / var(--frame-zoom)) calc(8.5px / var(--frame-zoom));
  }
</style>
