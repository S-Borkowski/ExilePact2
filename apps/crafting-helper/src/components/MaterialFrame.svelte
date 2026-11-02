<script lang="ts">
  // The Exile frame, drawn as the quest tracker's MaterialFrame draws it:
  // the steel/masonry kit's neutral steel PNG, unchanged, as a nine-slice
  // border (224 source px per side, 48 px corners, the centre left empty).
  // Decorative only: hidden from assistive technology, never a target. It
  // is fixed to the window, not to a scroller, and its visible art meets the
  // window's edge: each side's outset carries the PNG's own transparent
  // padding past the edge. That padding is 42 / 44 / 39 / 64 source px
  // (left, top, right, bottom), 9 / 9.4 / 8.4 / 13.7 px at 48 px; each
  // outset stays a little under it, so no art pixel is cut. These windows
  // have no in-app zoom, so the frame needs no zoom division. Only the
  // Exile theme draws it (the pages leave it out under Claude).
  import frame from '../../theme/materials/raster/frame-neutral-steel.png';
</script>

<div class="material-art" aria-hidden="true" style:--frame-image={`url("${frame}")`}></div>

<style>
  .material-art {
    position: fixed;
    z-index: 1;
    inset: 0;
    pointer-events: none;
    border: 0 solid transparent;
    border-image-source: var(--frame-image);
    border-image-slice: 224;
    border-image-width: 48px;
    border-image-repeat: stretch;
    /* Top, right, bottom, left: just under the PNG's padding at 48 px. */
    border-image-outset: 9px 8px 13.5px 8.5px;
  }
</style>
