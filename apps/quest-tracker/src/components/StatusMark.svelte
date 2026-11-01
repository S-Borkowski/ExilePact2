<script lang="ts">
  // Exile distinguishes quest states by diamond fill, outline and pattern;
  // Claude keeps its original symbols. Both expose the same single status name.
  //
  // The Exile diamonds are the steel/masonry kit's six finished status marks
  // (`docs/quest-tracker/design/steel-masonry-kit/components/status-*.svg`),
  // their drawing copied as live markup: same paths, paint and clip, without
  // the kit file's font styles or its own accessible name. One change from
  // the kit, on the owner's word (Live 2, 2026-11-02): a status colour fills
  // only the inside. The ring keeps the kit's stroke and is never filled; in
  // progress and done carry the kit's fill colour on an inner diamond, with a
  // gap of the well between it and the ring. The mark around
  // the svg carries the one name; the svg is decoration. The skipped hatch's
  // clip gets an id of its own per mark, so two skipped rows never share one.
  import type { QuestStatus } from '../ipc';
  import { STATUS_ICON, STATUS_LABEL } from '../labels';
  import Icon from './Icon.svelte';

  let { status, exile = false }: { status: QuestStatus; exile?: boolean } = $props();
  const uid = $props.id();
  const clipId = `status-skip-clip-${uid}`;
  /** The skipped hatch: 13 parallel strokes, 4 apart, clipped to the inner diamond. */
  const HATCH = [-24, -20, -16, -12, -8, -4, 0, 4, 8, 12, 16, 20, 24];
</script>

{#if exile}
  <span class="status-mark diamond-mark" data-status={status} role="img" aria-label={STATUS_LABEL[status]} title={STATUS_LABEL[status]}>
    <svg class="status-diamond" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
      {#if status === 'in_progress'}
        <path d="M12 2 22 12 12 22 2 12Z" fill="none" stroke="#B8C0C5" stroke-width="1.4" />
        <path d="M12 5 19 12 12 19 5 12Z" fill="#E6E5DE" />
        <path d="M12 5 19 12 12 11 5 12Z" fill="#FFFFFF" opacity=".42" />
      {:else if status === 'available'}
        <path d="M12 2 22 12 12 22 2 12Z" fill="none" stroke="#D0BF94" stroke-width="1.4" />
        <path d="M12 5 19 12 12 19 5 12Z" fill="none" stroke="#D0BF94" stroke-width=".8" />
      {:else if status === 'locked'}
        <path d="M12 2 22 12 12 22 2 12Z" fill="none" stroke="#69747C" stroke-width="1.4" />
      {:else if status === 'done'}
        <path d="M12 2 22 12 12 22 2 12Z" fill="none" stroke="#B8C0C5" stroke-width="1.4" />
        <path d="M12 5 19 12 12 19 5 12Z" fill="#818D95" />
        <path d="M12 5 19 12 12 19 5 12Z" fill="none" stroke="#D2D7DA" stroke-width=".8" />
      {:else if status === 'skipped'}
        <path d="M12 2 22 12 12 22 2 12Z" fill="none" stroke="#8D979E" stroke-width="1.4" />
        <defs><clipPath id={clipId}><path d="M12 3 21 12 12 21 3 12Z" /></clipPath></defs>
        <g clip-path={`url(#${clipId})`} stroke="#9AA5AC" stroke-width="1">
          {#each HATCH as x (x)}<path d={`M${x} 0l24 24`} />{/each}
        </g>
      {:else}
        <path d="M12 2 22 12 12 22 2 12Z" fill="none" stroke="#B8C0C5" stroke-width="1.4" stroke-dasharray=".5 2.5" stroke-linecap="round" />
      {/if}
    </svg>
  </span>
{:else}
  <span class="status-mark" data-status={status}>
    <Icon id={STATUS_ICON[status]} label={STATUS_LABEL[status]} size="16px" />
  </span>
{/if}

<style>
  .status-mark {
    display: inline-flex;
    flex: none;
  }
  /* The kit's 24 px diamond in a 24 px status column, drawn at 22 px. */
  .diamond-mark {
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
  }
  .status-diamond {
    display: block;
    width: 22px;
    height: 22px;
  }
  [data-status='locked'] { color: var(--color-status-locked); }
  [data-status='available'] { color: var(--color-status-available); }
  [data-status='in_progress'] { color: var(--color-status-in-progress); }
  [data-status='done'] { color: var(--color-status-done); }
  [data-status='skipped'] { color: var(--color-status-skipped); }
  [data-status='unknown'] { color: var(--color-status-unknown); }
</style>
