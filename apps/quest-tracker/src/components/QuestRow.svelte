<script lang="ts">
  // One quest row: status mark, name, the manual-tick marker, the reward
  // text, the worth marker, and the next step under an in-progress quest.
  // In edit mode the row opens the quest's detail.
  import type { Send } from '../bridge';
  import type { QuestRow } from '../ipc';
  import Icon from './Icon.svelte';
  import StatusMark from './StatusMark.svelte';
  import WorthMark from './WorthMark.svelte';

  let { row, edit, send }: { row: QuestRow; edit: boolean; send: Send } = $props();

  const rewardText = $derived(row.rewards.map((r) => r.text).join(' · '));
</script>

{#snippet body()}
  <span class="line">
    <StatusMark status={row.status} />
    <span class="name">{row.name}</span>
    {#if row.needs_tick}
      <span class="manual" title="Needs a manual tick">
        <Icon id="manual-tick" label="Needs a manual tick" size="12px" />
      </span>
    {/if}
    <span class="spacer"></span>
    <WorthMark worth={row.worth} />
  </span>
  {#if rewardText}<span class="sub rewards">{rewardText}</span>{/if}
  {#if row.current_step}<span class="sub step">Next: {row.current_step}</span>{/if}
{/snippet}

{#if edit}
  <button type="button" class="row" data-quest={row.id} data-status={row.status} onclick={() => send('open_detail', { questId: row.id })}>
    {@render body()}
  </button>
{:else}
  <div class="row" data-quest={row.id} data-status={row.status}>
    {@render body()}
  </div>
{/if}

<style>
  .row {
    display: flex;
    flex-direction: column;
    gap: 2px;
    width: 100%;
    box-sizing: border-box;
    padding: calc(var(--space-row-pad) / 2) var(--space-row-pad);
    border: 0;
    border-radius: var(--radius-row);
    background: transparent;
    color: inherit;
    font: inherit;
    font-size: var(--size-row);
    text-align: left;
  }

  button.row {
    cursor: pointer;
  }

  button.row:hover,
  button.row:focus-visible {
    background: rgba(255, 255, 255, 0.06);
    outline: none;
  }

  .line {
    display: flex;
    align-items: center;
    gap: var(--space-unit);
  }

  .name {
    font-weight: var(--font-weight-row);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .manual {
    display: inline-flex;
    color: var(--color-text-muted);
  }

  .spacer {
    flex: 1;
  }

  .sub {
    padding-left: calc(16px + var(--space-unit));
    font-size: var(--size-caption);
    color: var(--color-text-secondary);
  }

  .step {
    color: var(--color-status-in-progress);
  }

  [data-status='done'] .name,
  [data-status='skipped'] .name {
    color: var(--color-text-muted);
  }
</style>
