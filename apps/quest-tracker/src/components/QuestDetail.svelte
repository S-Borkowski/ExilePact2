<script lang="ts">
  // A quest's detail: back to the act, the title, giver and zones, the manual
  // note, the worth reason, every step with its leading mark (in edit mode the
  // step's one toggle, the hand tick), every reward as a chip,
  // the poe2db source, and in edit mode the actions. Reset asks twice: the
  // first click turns it into "Confirm reset", filled, with a 3 s countdown.
  // A quest or step done by inference (`inferred`) carries the "probably
  // done" mark after its status or step mark. An inferred step has no hand
  // tick in edit mode either: its leading mark is plain, and a click on it
  // sends nothing (owner: it "stays untickable"). A step the player ticked is
  // not inferred and keeps its toggle; Mark done / Unmark done stays, as the
  // player's correction when the inference is wrong.
  import { onDestroy } from 'svelte';
  import type { Send } from '../bridge';
  import type { Detection, QuestAction, QuestDetail, StepPhase, StepView } from '../ipc';
  import { STEP_ICON, STEP_LABEL, WORTH_LABEL } from '../labels';
  import Icon from './Icon.svelte';
  import InferredMark from './InferredMark.svelte';
  import StatusMark from './StatusMark.svelte';
  import WorthMark from './WorthMark.svelte';

  const CONFIRM_MS = 3000;

  let { detail, edit, send }: { detail: QuestDetail; edit: boolean; send: Send } = $props();

  const row = $derived(detail.row);
  const facts = $derived(
    [detail.giver ? `Giver: ${detail.giver}` : null, detail.zones.length > 0 ? `${detail.zones.length === 1 ? 'Zone' : 'Zones'}: ${detail.zones.join(', ')}` : null]
      .filter(Boolean)
      .join(' · '),
  );
  const MANUAL_NOTE: Record<Detection, string> = {
    auto: '',
    partial: 'Partly tracked by the log: tick the steps it misses',
    manual: 'Manual: not tracked by the log; tick steps as you go',
  };

  let confirming = $state(false);
  let confirmTimer: ReturnType<typeof setTimeout> | undefined;

  function act(action: QuestAction): void {
    send('quest_action', { questId: row.id, action });
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

  // The leading mark draws what the step reads as: the check in a box when the
  // log finished it or it was ticked by hand, else its own phase. In edit mode
  // it is also the step's one toggle (the hand tick), unless the step is inferred.
  function markPhase(step: StepView): StepPhase {
    return step.state === 'done' || step.ticked ? 'done' : step.state;
  }
</script>

{#snippet mark(phase: StepPhase, labelled: boolean)}
  {#if phase === 'pending'}
    <span class="box" role={labelled ? 'img' : undefined} aria-label={labelled ? STEP_LABEL.pending : undefined} aria-hidden={labelled ? undefined : 'true'}></span>
  {:else}
    <Icon id={STEP_ICON[phase]} label={labelled ? STEP_LABEL[phase] : null} size="14px" />
  {/if}
{/snippet}

<div class="detail" data-quest={row.id}>
  <div class="content">
    {#if edit}
      <button type="button" class="back" onclick={() => send('open_detail', { questId: null })}>
        <Icon id="ui-chevron-left" size="12px" /> Act {row.act}
      </button>
    {:else}
      <span class="back">Act {row.act}</span>
    {/if}

    <div class="head">
      <StatusMark status={row.status} />
      {#if row.inferred}<InferredMark />{/if}
      <h2 class="name">{row.name}</h2>
    </div>
    {#if facts}<p class="caption">{facts}</p>{/if}
    {#if row.needs_tick}
      <p class="caption manual"><Icon id="manual-tick" size="12px" /> {MANUAL_NOTE[row.detection] || MANUAL_NOTE.manual}</p>
    {/if}

    <div class="worth" data-worth={row.worth}>
      <WorthMark worth={row.worth} />
      <div class="worth-body">
        <p class="worth-label">{WORTH_LABEL[row.worth]}</p>
        <p class="reason">{detail.worth_reason}</p>
      </div>
    </div>
  </div>

  <h3 class="group-label">Steps</h3>
  <ol class="steps">
    {#each detail.steps as step (step.id)}
      {@const phase = markPhase(step)}
      <li class="step" data-state={step.state} data-step={step.id} data-ticked={step.ticked}>
        {#if edit && !step.inferred}
          <button
            type="button"
            class="mark"
            aria-pressed={step.ticked}
            aria-label={`${step.title}: ${STEP_LABEL[phase]}. Tick by hand`}
            title={step.ticked ? 'Ticked by hand: click to untick' : 'Tick by hand'}
            onclick={() => send('tick_step', { questId: row.id, stepId: step.id, ticked: !step.ticked })}
          >
            {@render mark(phase, false)}
          </button>
        {:else}
          <span class="mark">{@render mark(phase, true)}</span>
        {/if}
        {#if step.inferred}<InferredMark />{/if}
        <span class="step-body">
          <span class="step-title">{step.title}</span>
          {#if step.text}<span class="step-text">{step.text}</span>{/if}
        </span>
        {#if step.ticked}<span class="ticked">ticked</span>{/if}
      </li>
    {/each}
  </ol>

  {#if row.rewards.length > 0}
    <h3 class="group-label">Rewards</h3>
    <ul class="chips">
      {#each row.rewards as reward, i (i)}
        <li class="chip" data-category={reward.category} data-icon={reward.icon}>
          <Icon id={reward.icon} size="20px" />
          <span>{reward.text}</span>
        </li>
      {/each}
    </ul>
  {/if}

  <p class="caption source">
    {#if edit}
      <button type="button" class="link" onclick={() => send('open_source', { questId: row.id })}>
        Open on poe2db <Icon id="ui-external-link" size="12px" />
      </button>
    {:else}
      <span>poe2db</span>
    {/if}
    <span>fetched {detail.fetched}</span>
  </p>

  {#if edit}
    <div class="actions">
      {#if row.status === 'done'}
        <button type="button" class="btn cut primary" onclick={() => act('unmark_done')}>Unmark done</button>
      {:else}
        <button type="button" class="btn cut primary" onclick={() => act('mark_done')}>Mark done</button>
      {/if}
      {#if row.status === 'skipped'}
        <button type="button" class="btn cut" onclick={() => act('unskip')}>Unskip</button>
      {:else}
        <button type="button" class="btn cut" onclick={() => act('skip')}>Skip</button>
      {/if}
      <button type="button" class="btn cut danger" class:confirm={confirming} onclick={reset}>
        <Icon id="ui-refresh" size="14px" />
        {confirming ? 'Confirm reset' : 'Reset'}
        {#if confirming}<span class="countdown" style:--countdown="var(--motion-confirm-ms)"></span>{/if}
      </button>
    </div>
  {/if}
</div>

<style>
  .detail {
    display: flex;
    flex-direction: column;
  }

  .content {
    display: flex;
    flex-direction: column;
    gap: var(--space-unit);
    padding: var(--space-section-gap) var(--space-row-pad) 0;
  }

  .back {
    display: inline-flex;
    align-self: flex-start;
    align-items: center;
    gap: var(--space-unit);
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--color-text-muted);
    font-family: var(--font-family);
    font-size: var(--size-caption);
    font-weight: var(--font-weight-row);
    line-height: var(--line-caption);
  }

  button.back {
    cursor: pointer;
  }

  button.back:hover,
  button.back:focus-visible {
    color: var(--color-text-primary);
    outline: none;
  }

  .head {
    display: flex;
    align-items: center;
    gap: var(--space-section-gap);
  }

  .name {
    margin: 0;
    color: var(--color-text-primary);
    font-size: 20px;
    font-weight: var(--font-weight-title);
    line-height: 24px;
  }

  .manual {
    display: flex;
    align-items: center;
    gap: var(--space-unit);
  }

  .worth {
    display: flex;
    align-items: flex-start;
    gap: var(--space-section-gap);
    margin-top: var(--space-unit);
  }

  .worth-body {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .worth p {
    margin: 0;
  }

  .worth-label {
    font-size: var(--size-label);
    font-weight: var(--font-weight-row);
    line-height: var(--line-caption);
  }

  .worth[data-worth='yes'] .worth-label {
    color: var(--color-worth-gold);
  }

  .worth[data-worth='optional'] .worth-label,
  .worth[data-worth='no'] .worth-label {
    color: var(--color-text-secondary);
  }

  .reason {
    color: var(--color-text-secondary);
    font-size: var(--size-step);
    font-weight: 400;
    line-height: var(--line-step);
  }

  .steps,
  .chips {
    display: flex;
    margin: 0;
    padding: 0 var(--space-row-pad);
    list-style: none;
  }

  .steps {
    flex-direction: column;
    gap: 2px;
  }

  .step {
    display: flex;
    align-items: flex-start;
    gap: var(--space-section-gap);
    min-height: 20px;
    padding: 2px 0;
    color: var(--color-text-muted);
  }

  .mark {
    display: inline-flex;
    flex: none;
    align-items: center;
    justify-content: center;
    margin-top: 2px;
  }

  /* Edit mode: the mark is the hand tick. The hit area is 24x24 around the
     14 px glyph; the negative margins keep the glyph where play mode draws it. */
  button.mark {
    box-sizing: border-box;
    min-width: 24px;
    min-height: 24px;
    margin: -3px -5px;
    padding: 0;
    border: var(--border-width) solid transparent;
    background: transparent;
    color: inherit;
    cursor: pointer;
  }

  button.mark:hover,
  button.mark:focus-visible {
    border-color: var(--color-edit-accent);
    color: var(--color-edit-accent);
    outline: none;
  }

  /* The "probably done" mark sits 4 px after the status or step mark (the
     row's gap less 4). In a step it centres on the 14 px mark, whose top is
     2 px down in both modes: 2 + (14 - 10) / 2. */
  .head > :global(.inferred-mark),
  .step > :global(.inferred-mark) {
    margin-left: calc(var(--space-unit) - var(--space-section-gap));
  }

  .step > :global(.inferred-mark) {
    margin-top: 4px;
  }

  .box {
    flex: none;
    box-sizing: border-box;
    width: 14px;
    height: 14px;
    border: 1.5px solid var(--color-text-secondary);
  }

  .step[data-state='current'] {
    color: var(--color-status-in-progress);
  }

  .step-body {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
  }

  .step-title {
    font-size: var(--size-step);
    font-weight: 400;
    line-height: var(--line-step);
  }

  .step[data-state='pending'] .step-title {
    color: var(--color-text-secondary);
  }

  .step[data-state='current'] .step-title {
    color: var(--color-text-primary);
  }

  .step[data-state='done'] .step-title {
    color: var(--color-text-muted);
  }

  .step-text,
  .ticked {
    color: var(--color-text-muted);
    font-size: var(--size-caption);
    line-height: var(--line-caption);
  }

  .chips {
    flex-wrap: wrap;
    gap: var(--space-unit);
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-height: 28px;
    box-sizing: border-box;
    padding: var(--space-unit) var(--space-section-gap);
    background: var(--color-panel-raised);
    color: var(--color-text-primary);
    font-size: var(--size-label);
    font-weight: var(--font-weight-row);
    line-height: var(--line-caption);
  }

  .chip :global(.icon) {
    color: var(--color-text-secondary);
  }

  .chip[data-icon='reward-res-fire'] :global(.icon) {
    color: var(--color-res-fire);
  }

  .chip[data-icon='reward-res-cold'] :global(.icon) {
    color: var(--color-res-cold);
  }

  .chip[data-icon='reward-res-lightning'] :global(.icon) {
    color: var(--color-res-lightning);
  }

  .chip[data-icon='reward-res-chaos'] :global(.icon) {
    color: var(--color-res-chaos);
  }

  .chip[data-icon='reward-spirit'] :global(.icon),
  .chip[data-icon='reward-passive'] :global(.icon) {
    color: var(--color-worth-gold);
  }

  .source {
    display: flex;
    align-items: center;
    gap: var(--space-section-gap);
    padding: var(--space-section-gap) var(--space-row-pad) 0;
  }

  .actions {
    display: flex;
    gap: var(--space-section-gap);
    margin-top: var(--space-section-gap);
    padding: var(--space-section-gap) var(--space-row-pad) 0;
    border-top: var(--border-width) solid var(--color-panel-divider);
  }

  .actions .primary {
    flex: 1;
  }
</style>
