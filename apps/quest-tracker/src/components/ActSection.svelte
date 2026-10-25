<script lang="ts">
  // An act's header ("Act 1", progress, chevron) and, when expanded, its rows.
  import type { Send } from '../bridge';
  import type { ActView } from '../ipc';
  import Icon from './Icon.svelte';
  import QuestRow from './QuestRow.svelte';

  let { act, edit, send }: { act: ActView; edit: boolean; send: Send } = $props();
</script>

{#snippet header()}
  <Icon id={act.expanded ? 'ui-collapse' : 'ui-expand'} size="12px" />
  <span class="title">{act.title}</span>
  {#if act.has_available}<span class="dot" title="Has available quests">●</span>{/if}
  <span class="spacer"></span>
  <span class="progress">{act.done}/{act.total}</span>
{/snippet}

<section class="act" data-act={act.act}>
  {#if edit}
    <button type="button" class="act-header" aria-expanded={act.expanded} onclick={() => send('toggle_act', { act: act.act })}>
      {@render header()}
    </button>
  {:else}
    <div class="act-header">{@render header()}</div>
  {/if}
  {#if act.expanded}
    <div class="rows">
      {#each act.quests as row (row.id)}
        <QuestRow {row} {edit} {send} />
      {/each}
    </div>
  {/if}
</section>

<style>
  .act-header {
    display: flex;
    align-items: center;
    gap: var(--space-unit);
    width: 100%;
    box-sizing: border-box;
    padding: var(--space-unit) var(--space-row-pad);
    border: 0;
    background: transparent;
    color: var(--color-text-secondary);
    font: inherit;
    font-size: var(--size-caption);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    text-align: left;
  }

  button.act-header {
    cursor: pointer;
  }

  .title {
    font-weight: var(--font-weight-title);
  }

  .dot {
    color: var(--color-status-available);
    font-size: 8px;
  }

  .spacer {
    flex: 1;
  }

  .progress {
    font-variant-numeric: tabular-nums;
  }
</style>
