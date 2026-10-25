<script lang="ts">
  // One icon id of the brief: the harvested SVG when present (coloured by
  // `currentColor`), else its text glyph. With a label it is an image with
  // that name; without one it is decoration beside text that says the same.
  import { iconGlyph, iconSvg } from '../theme';

  let { id, label = null, size = '1em' }: { id: string; label?: string | null; size?: string } = $props();

  const svg = $derived(iconSvg(id));
</script>

{#if label}
  <span class="icon" data-icon={id} role="img" aria-label={label} style:--icon-size={size}>
    {#if svg}{@html svg}{:else}<span class="glyph">{iconGlyph(id)}</span>{/if}
  </span>
{:else}
  <span class="icon" data-icon={id} aria-hidden="true" style:--icon-size={size}>
    {#if svg}{@html svg}{:else}<span class="glyph">{iconGlyph(id)}</span>{/if}
  </span>
{/if}

<style>
  .icon {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    flex: none;
    width: var(--icon-size);
    height: var(--icon-size);
    line-height: 1;
  }

  .icon :global(svg) {
    width: 100%;
    height: 100%;
  }

  .glyph {
    font-size: var(--icon-size);
    line-height: 1;
  }
</style>
