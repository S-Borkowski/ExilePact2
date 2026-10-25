<script lang="ts">
  // A quest's detail: every step with its tick, every reward, giver and
  // zones, the worth reason, the poe2db source, and in edit mode the actions.
  // Reset asks twice: the first click relabels it "Confirm reset" for 3 s.
  import { onDestroy } from 'svelte';
  import type { Send } from '../bridge';
  import type { QuestAction, QuestDetail } from '../ipc';
  import { STEP_ICON, STEP_LABEL, WORTH_LABEL } from '../labels';
  import Icon from './Icon.svelte';
  import StatusMark from './StatusMark.svelte';
  import WorthMark from './WorthMark.svelte';

  const CONFIRM_MS = 3000;

  let {
    detail,
    edit,
    send,
    onAction = () => {},
  }: { detail: QuestDetail; edit: boolean; send: Send; onAction?: (action: QuestAction) => void } = $props();

  const row = $derived(detail.row);
  let confirming = $state(false);
  let confirmTimer: ReturnType<typeof setTimeout> | undefined;

  function act(action: QuestAction): void {
    send('quest_action', { questId: row.id, action });
    onAction(action);
  }

  function reset(): void {
    if (!confirming) {
      confirming = true;
      confirmTimer = setTimeout(() => (confirming = false), CONFIRM_MS);
      return;
    }
    clearTimeout(confirmTimer);
    confirming = false;
    act('reset');
  }

  onDestroy(() => clearTimeout(confirmTimer));
</script>

<div class="detail" data-quest={row.id}>
  <div class="head">
    <StatusMark status={row.status} />
    <span class="name">{row.name}</span>
    <span class="spacer"></span>
    <WorthMark worth={row.worth} />
    {#if edit}
      <button type="button" class="icon-button" aria-label="Close" title="Close" onclick={() => send('open_detail', { questId: null })}>
        <Icon id="ui-close" />
      </button>
    {/if}
  </div>

  <dl class="facts">
    {#if detail.giver}<dt>Giver</dt><dd>{detail.giver}</dd>{/if}
    {#if detail.zones.length > 0}<dt>Zones</dt><dd>{detail.zones.join(', ')}</dd>{/if}
    <dt>{WORTH_LABEL[row.worth]}</dt>
    <dd>{detail.worth_reason}</dd>
    {#if row.needs_tick}
      <dt><Icon id="manual-tick" size="12px" /> Manual</dt>
      <dd>The log does not record all of this quest: tick its steps by hand.</dd>
    {/if}
  </dl>

  {#if row.rewards.length > 0}
    <ul class="rewards">
      {#each row.rewards as reward, i (i)}
        <li data-category={reward.category}><Icon id={reward.icon} size="20px" /> {reward.text}</li>
      {/each}
    </ul>
  {/if}

  <ol class="steps">
    {#each detail.steps as step (step.id)}
      <li class="step" data-state={step.state}>
        <Icon id={STEP_ICON[step.state]} label={STEP_LABEL[step.state]} size="14px" />
        <span class="step-body">
          <span class="step-title">{step.title}</span>
          {#if step.text}<span class="step-text">{step.text}</span>{/if}
        </span>
        {#if edit}
          <input
            type="checkbox"
            checked={step.ticked}
            aria-label={`Ticked by hand: ${step.title}`}
            onchange={() => send('tick_step', { questId: row.id, stepId: step.id, ticked: !step.ticked })}
          />
        {:else if step.ticked}
          <span class="ticked">ticked</span>
        {/if}
      </li>
    {/each}
  </ol>

  <div class="source">
    {#if edit}
      <button type="button" class="link" onclick={() => send('open_source', { questId: row.id })}>
        poe2db <Icon id="ui-external-link" size="12px" />
      </button>
    {:else}
      <span>poe2db</span>
    {/if}
    <span class="fetched">fetched {detail.fetched}</span>
  </div>

  {#if edit}
    <div class="actions">
      {#if row.status === 'done'}
        <button type="button" class="primary" onclick={() => act('unmark_done')}>Unmark done</button>
      {:else}
        <button type="button" class="primary" onclick={() => act('mark_done')}>Mark done</button>
      {/if}
      {#if row.status === 'skipped'}
        <button type="button" onclick={() => act('unskip')}>Unskip</button>
      {:else}
        <button type="button" onclick={() => act('skip')}>Skip</button>
      {/if}
      <button type="button" class="danger" onclick={reset}>{confirming ? 'Confirm reset' : 'Reset'}</button>
    </div>
  {/if}
</div>

<style>
  .detail {
    display: flex;
    flex-direction: column;
    gap: var(--space-section-gap);
    padding: var(--space-unit) var(--space-row-pad);
    font-size: var(--size-step);
  }

  .head {
    display: flex;
    align-items: center;
    gap: var(--space-unit);
  }

  .name {
    font-size: var(--size-row);
    font-weight: var(--font-weight-title);
  }

  .spacer {
    flex: 1;
  }

  .facts {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 2px var(--space-section-gap);
    margin: 0;
  }

  dt {
    color: var(--color-text-muted);
    font-size: var(--size-caption);
  }

  dd {
    margin: 0;
    color: var(--color-text-secondary);
  }

  .rewards,
  .steps {
    display: flex;
    flex-direction: column;
    gap: var(--space-unit);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .rewards li {
    display: flex;
    align-items: center;
    gap: var(--space-unit);
  }

  .step {
    display: flex;
    align-items: flex-start;
    gap: var(--space-unit);
  }

  .step[data-state='done'] {
    color: var(--color-status-done);
  }

  .step[data-state='current'] {
    color: var(--color-status-in-progress);
  }

  .step[data-state='pending'] {
    color: var(--color-text-secondary);
  }

  .step-body {
    display: flex;
    flex: 1;
    flex-direction: column;
  }

  .step-title {
    color: var(--color-text-primary);
  }

  .step-text,
  .ticked,
  .fetched {
    color: var(--color-text-muted);
    font-size: var(--size-caption);
  }

  .source {
    display: flex;
    align-items: center;
    gap: var(--space-section-gap);
  }

  .actions {
    display: flex;
    gap: var(--space-unit);
  }

  button {
    padding: 4px var(--space-section-gap);
    border: var(--border-width) solid var(--color-panel-border);
    border-radius: var(--radius-button);
    background: transparent;
    color: var(--color-text-primary);
    font: inherit;
    font-size: var(--size-caption);
    cursor: pointer;
  }

  button.primary {
    border-color: var(--color-text-primary);
  }

  button.danger {
    border-color: var(--color-res-fire);
    color: var(--color-res-fire);
  }

  button.link,
  .icon-button {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    border: 0;
    padding: 0;
    color: var(--color-text-secondary);
  }

  .icon-button {
    width: 24px;
    height: 24px;
    justify-content: center;
  }
</style>
