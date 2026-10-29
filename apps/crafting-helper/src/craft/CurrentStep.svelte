<script lang="ts">
  // Current step (`ui-design.md` § "Mode guiding"): the route's name and the
  // guide's status as a word, the last outcome where the player looks after
  // a re-grab, then for active and stuck the step's card with its branch
  // lines as Rust wrote them, the reasons when stuck or replan, and "The
  // craft is done." when done. The page judges nothing: the last outcome is
  // the newest History entry, as sent.
  import type { GuidingView } from '../ipc';
  import StepFacts from './StepFacts.svelte';
  import { OUTCOME_LABELS, STATUS_LABELS } from './words';

  let { guiding, hotkey }: { guiding: GuidingView; hotkey: string } = $props();

  const uid = $props.id();
  const last = $derived(guiding.history.length > 0 ? guiding.history[guiding.history.length - 1] : null);
</script>

<section class="guide" aria-labelledby="{uid}-title">
  <div class="title-row">
    <h2 class="route-title" id="{uid}-title" tabindex="-1" data-mode-heading>{guiding.route_name}</h2>
    <span class="guide-status" data-status={guiding.status}>{STATUS_LABELS[guiding.status]}</span>
  </div>

  {#if last && guiding.status !== 'done'}
    <p class="last"><span class="label">Last:</span> <span class="result" data-result={last.result}>{OUTCOME_LABELS[last.result]}</span> <span class="summary">{last.summary}</span></p>
  {/if}

  {#if guiding.step && (guiding.status === 'active' || guiding.status === 'stuck')}
    {@const current = guiding.step}
    <div class="card">
      <p class="step-head"><span class="step-no">Step {current.step.number}</span> <span class="try">try {current.try_number}</span></p>
      <p class="instruction">{current.step.instruction}</p>
      <StepFacts step={current.step} />
      <ul class="branches">
        <li class="branch" data-branch="hit">{current.on_hit}</li>
        <li class="branch" data-branch="miss">{current.on_miss}</li>
      </ul>
      <p class="prompt">
        Take this step in the game, copy the item with <kbd>Ctrl+Alt+C</kbd>, then press <kbd>{hotkey}</kbd> or <kbd>Ctrl+V</kbd> here.
      </p>
    </div>
  {/if}

  {#if guiding.status === 'stuck' || guiding.status === 'replan'}
    <div class="why">
      <p class="why-title">Why:</p>
      <ul class="why-list">
        {#each guiding.reasons as reason, i (i)}<li>{reason}</li>{/each}
      </ul>
      {#if guiding.status === 'replan'}<p class="replan-hint">End the craft to plan again from this item.</p>{/if}
    </div>
  {/if}

  {#if guiding.status === 'done'}
    <div class="done">
      <p class="done-line">The craft is done.</p>
      {#if last}<p class="summary">{last.summary}</p>{/if}
    </div>
  {/if}
</section>

<style>
  .guide {
    display: flex;
    flex-direction: column;
    gap: calc(var(--space-unit) * 3);
  }

  p {
    margin: 0;
    font-size: var(--size-step);
    line-height: var(--line-step);
  }

  .title-row {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: calc(var(--space-unit) * 3);
  }

  .route-title {
    margin: 0;
    color: var(--color-text-primary);
    font-size: var(--size-title);
    line-height: var(--line-title);
    font-weight: 700;
  }

  .guide-status {
    padding: 0 calc(var(--space-unit) * 1.5);
    border: var(--border-width) solid currentColor;
    color: var(--color-status-in-progress);
    font-size: var(--size-caption);
    line-height: var(--line-caption);
    font-weight: 700;
  }

  .guide-status[data-status='stuck'],
  .guide-status[data-status='replan'] {
    color: var(--error);
  }

  .guide-status[data-status='done'] {
    color: var(--want);
  }

  .label {
    color: var(--color-text-muted);
  }

  .last .result {
    font-weight: 700;
    color: var(--want);
  }

  .last .result[data-result='miss'],
  .last .result[data-result='unexpected'] {
    color: var(--error);
  }

  .last .summary {
    color: var(--color-text-secondary);
  }

  .card {
    display: flex;
    flex-direction: column;
    gap: calc(var(--space-unit) * 1.5);
    padding: calc(var(--space-unit) * 3) calc(var(--space-unit) * 4);
    border: var(--border-width) solid var(--color-panel-border);
    background: var(--color-panel-sunken);
  }

  .step-head {
    color: var(--color-text-muted);
  }

  .step-no {
    color: var(--color-text-secondary);
    font-weight: 700;
  }

  /* The instruction is what the player acts on: larger than the body text. */
  .instruction {
    color: var(--color-text-primary);
    font-size: var(--size-row);
    line-height: var(--line-row);
    font-weight: 700;
  }

  .branches {
    margin: var(--space-unit) 0 0;
    padding: 0;
    list-style: none;
  }

  .branch {
    padding: var(--space-unit) 0;
    border-top: var(--border-width) solid var(--color-panel-divider);
    color: var(--color-text-primary);
    font-size: var(--size-step);
    line-height: var(--line-step);
  }

  .prompt {
    margin-top: var(--space-unit);
    color: var(--color-text-secondary);
  }

  .why-title {
    color: var(--color-text-primary);
    font-weight: 700;
  }

  .why-list {
    margin: var(--space-unit) 0;
    padding-left: calc(var(--space-unit) * 5);
    color: var(--color-text-secondary);
    font-size: var(--size-step);
    line-height: var(--line-step);
  }

  .replan-hint {
    color: var(--color-text-primary);
  }

  .done-line {
    color: var(--want);
    font-size: var(--size-row);
    line-height: var(--line-row);
    font-weight: 700;
  }

  .done .summary {
    color: var(--color-text-secondary);
  }
</style>
