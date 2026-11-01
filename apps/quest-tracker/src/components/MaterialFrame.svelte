<script lang="ts">
  import frame from '../../theme/materials/raster/journal-frame.png';
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
    border-image-slice: 208 190 208 190;
    border-image-width: 34px 30px;
    border-image-repeat: stretch round;
    border-image-outset: 0;
  }
  .plaque { border-image-width: 20px 18px; }
  @media (max-height: 320px), (max-width: 300px) {
    .material-art:not(.plaque) { border-image-width: 28px 12px 14px; }
  }
</style>
