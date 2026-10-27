<script lang="ts">
  // An act's header (chevron, "Act 1" in small capitals, the available mark,
  // "n available ·" and "done/total done") on the raised band and, when
  // expanded, its rows.
  import type { Send } from '../bridge';
  import type { ActView } from '../ipc';
  import Icon from './Icon.svelte';
  import QuestRow from './QuestRow.svelte';

  let { act, edit, send }: { act: ActView; edit: boolean; send: Send } = $props();

  // Counted from the act's rows, which the view carries only while the act
  // is expanded; a collapsed act shows the mark (`has_available`) alone.
  const available = $derived(act.quests.filter((q) => q.status === 'available').length);
</script>

{#snippet header()}
  <Icon id={act.expanded ? 'ui-chevron-down' : 'ui-chevron'} size="12px" />
  <span class="title">{act.title}</span>
  <span class="spacer"></span>
  {#if act.has_available || available > 0}
    <span class="available"><Icon id="status-available" label="Has available quests" size="12px" /></span>
  {/if}
  {#if available > 0}
    <span class="available-count">{available} available ·</span>
  {/if}
  <span class="progress">{act.done}/{act.total} done</span>
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
    gap: 6px;
    box-sizing: border-box;
    width: 100%;
    min-height: var(--size-act-header);
    padding: 0 var(--space-row-pad);
    border: 0;
    background: var(--color-panel-raised);
    color: var(--color-text-secondary);
    font-family: var(--font-family);
    text-align: left;
  }

  button.act-header {
    cursor: pointer;
  }

  button.act-header:hover,
  button.act-header:focus-visible {
    box-shadow: inset 0 0 0 var(--border-width) var(--color-hover-outline);
    color: var(--color-text-primary);
    outline: none;
  }

  .title {
    color: var(--color-text-primary);
    font-family: var(--font-family-caps);
    font-size: var(--size-act);
    font-weight: var(--font-weight-title);
    line-height: var(--line-caption);
    letter-spacing: 0.03em;
  }

  .spacer {
    flex: 1;
  }

  .available {
    display: inline-flex;
    color: var(--color-status-available);
  }

  .available-count,
  .progress {
    color: var(--color-text-muted);
    font-size: var(--size-caption);
    line-height: var(--line-caption);
    letter-spacing: 0.2px;
    font-variant-numeric: tabular-nums;
  }

  .rows {
    display: flex;
    flex-direction: column;
  }
</style>
