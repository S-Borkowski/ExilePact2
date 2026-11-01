<script lang="ts">
  // The Exile frame: the steel/masonry kit's neutral steel PNG, unchanged, as
  // a nine-slice border (224 source px per side, 48 px corners, 8 px out into
  // the overlay's transparent margin, the centre left empty). Decorative only:
  // hidden from assistive technology, never a target. It keeps one size on
  // screen at every scale, as the 12 px margin does, so only the content zooms
  // (owner, 2026-11-01). The ordinary Settings window has no transparent
  // margin: there (`plaque`) the frame sits wholly inside the window.
  import frame from '../../theme/materials/raster/frame-neutral-steel.png';
  let { plaque = false }: { plaque?: boolean } = $props();
</script>

<div class="material-art" class:plaque aria-hidden="true" style:--frame-image={`url("${frame}")`}></div>

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
    border-image-outset: calc(8px / var(--frame-zoom));
  }
  .plaque { border-image-outset: 0; }
</style>
