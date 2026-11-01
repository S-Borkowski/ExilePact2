<script lang="ts">
  import frame from '../../theme/materials/raster/tracker-steel-frame.png';
  let { plaque = false }: { plaque?: boolean } = $props();
</script>

<div class="material-art" class:plaque aria-hidden="true" style:--frame-image={`url("${frame}")`}></div>

<style>
  .material-art {
    position: absolute;
    z-index: 1;
    inset: 0;
    pointer-events: none;
    border: 0 solid transparent;
    border-image-source: var(--frame-image);
    border-image-slice: 224;
    border-image-width: 64px;
    border-image-repeat: stretch;
    border-image-outset: 12px;
  }
  .plaque { border-image-outset: 0; }
</style>
