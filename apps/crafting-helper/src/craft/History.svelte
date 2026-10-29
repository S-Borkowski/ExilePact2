<script lang="ts">
  // History (`ui-design.md` § "Mode guiding"): one entry per observed
  // change, oldest first, each with its step, the outcome as a word and
  // Rust's summary. Not a live region: the status line announces each
  // outcome.
  import type { HistoryEntry } from '../ipc';
  import { OUTCOME_LABELS } from './words';

  let { history }: { history: HistoryEntry[] } = $props();

  const uid = $props.id();
</script>

<section class="history" aria-labelledby="{uid}-title">
  <h2 class="section-title caps" id="{uid}-title">History</h2>
  {#if history.length === 0}
    <p class="empty">No step taken yet.</p>
  {:else}
    <ol>
      {#each history as entry, i (i)}
        <li>
          <p class="head"><span class="step-no">Step {entry.number}</span> <span class="instruction">{entry.instruction}</span></p>
          <p class="outcome"><span class="result" data-result={entry.result}>{OUTCOME_LABELS[entry.result]}</span> <span class="summary">{entry.summary}</span></p>
        </li>
      {/each}
    </ol>
  {/if}
</section>

<style>
  .history {
    display: flex;
    flex-direction: column;
    gap: calc(var(--space-unit) * 2);
  }

  p {
    margin: 0;
    font-size: var(--size-step);
    line-height: var(--line-step);
  }

  .empty {
    color: var(--color-text-muted);
  }

  ol {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li {
    padding: calc(var(--space-unit) * 1.5) 0;
    border-top: var(--border-width) solid var(--color-panel-divider);
  }

  .step-no {
    color: var(--color-text-primary);
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }

  .instruction {
    color: var(--color-text-muted);
  }

  .summary {
    color: var(--color-text-secondary);
  }

  /* The outcome is a word, set as an outlined label (as the guide's status)
     so it reads as the entry's mark before Rust's summary, which starts with
     the same word; Miss and Unexpected take the warning colour too. */
  .result {
    display: inline-block;
    padding: 0 var(--space-unit);
    border: var(--border-width) solid currentColor;
    color: var(--want);
    font-size: var(--size-caption);
    line-height: var(--line-caption);
    font-weight: 700;
  }

  .result[data-result='miss'],
  .result[data-result='unexpected'] {
    color: var(--error);
  }
</style>
