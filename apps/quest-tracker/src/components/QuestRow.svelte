<script lang="ts">
  // One quest row: status mark, name and the manual-tick marker, the short
  // reward labels, the worth marker, and the next step under an in-progress
  // quest. A quest done by inference (`inferred`) carries the "probably done"
  // mark right after its status mark; it is not a control. In progress: the active band with a 3 px bar inside the left edge.
  // Finished (done, skipped, locked): a compact, muted row. In edit mode the
  // row opens the quest's detail, with an outline on hover.
  //
  // A live change of this quest (`qt:row-change`, `../rowchange.ts`) plays
  // here: the completed step fades out, then the step now current (or, with
  // none, the row's title in its new status) fades in from 0.2 opacity,
  // rising 5 px. A second change restarts it from the start; under reduced
  // motion the change is instant. Changes from before the row was on screen
  // (its act folded, or filtered out) are not played: App announced them.
  import { onDestroy, untrack } from 'svelte';
  import type { Send } from '../bridge';
  import type { QuestRow, RowChange } from '../ipc';
  import { rewardShortText, rewardText } from '../labels';
  import { ROW_STEP_IN_MS, ROW_STEP_OUT_MS, type RowPhase, getRowChanges, prefersReducedMotion } from '../rowchange';
  import Icon from './Icon.svelte';
  import InferredMark from './InferredMark.svelte';
  import StatusMark from './StatusMark.svelte';
  import WorthMark from './WorthMark.svelte';

  let { row, edit, send }: { row: QuestRow; edit: boolean; send: Send } = $props();

  // The column shows the short labels; the game's full text is the title.
  const rewardShort = $derived(rewardShortText(row.rewards));
  const rewardFull = $derived(rewardText(row.rewards));
  const finished = $derived(row.status === 'done' || row.status === 'skipped' || row.status === 'locked');

  const changes = getRowChanges();
  // The last change this row played, or saw before it was shown.
  let played = changes?.latest ?? 0;
  let phase = $state<RowPhase>('idle');
  let leaving = $state<string | null>(null);
  // Keys the row's content, so a restart starts every animation from zero.
  let run = $state(0);
  let timers: Array<ReturnType<typeof setTimeout>> = [];

  function stopTimers(): void {
    for (const timer of timers) clearTimeout(timer);
    timers = [];
  }

  function after(ms: number, then: () => void): void {
    timers.push(setTimeout(then, ms));
  }

  function play(change: RowChange): void {
    stopTimers();
    run = change.id;
    leaving = null;
    phase = 'idle';
    if (prefersReducedMotion()) return;
    const enter = () => {
      leaving = null;
      phase = 'in';
      after(ROW_STEP_IN_MS, () => (phase = 'idle'));
    };
    if (change.step_done) {
      leaving = change.step_done;
      phase = 'out';
      after(ROW_STEP_OUT_MS, enter);
    } else {
      enter();
    }
  }

  $effect(() => {
    const change = changes?.byQuest.get(row.id);
    if (!change || change.id <= played) return;
    played = change.id;
    untrack(() => play(change));
  });

  onDestroy(stopTimers);
</script>

{#snippet content()}
  <StatusMark status={row.status} />
  <span class="title" class:entering={phase === 'in' && !row.current_step}>
    {#if row.inferred}<InferredMark />{/if}
    <span class="name">{row.name}</span>
    {#if row.needs_tick}
      <span class="manual" title="Needs a manual tick">
        <Icon id="manual-tick" label="Needs a manual tick" size="12px" />
      </span>
    {/if}
  </span>
  <span class="rewards" title={rewardFull || undefined}>{rewardShort}</span>
  <WorthMark worth={row.worth} />
  {#if phase === 'out' && leaving}
    <span class="step leaving" aria-hidden="true">{leaving}</span>
  {:else if row.current_step}
    <span class="step" class:entering={phase === 'in'}>{row.current_step}</span>
  {/if}
{/snippet}

{#snippet body()}
  {#key run}{@render content()}{/key}
{/snippet}

{#if edit}
  <button
    type="button"
    class="row"
    class:finished
    data-quest={row.id}
    data-status={row.status}
    data-worth={row.worth}
    data-change={phase === 'idle' ? undefined : phase}
    style:--row-out-ms={`${ROW_STEP_OUT_MS}ms`}
    style:--row-in-ms={`${ROW_STEP_IN_MS}ms`}
    onclick={() => send('open_detail', { questId: row.id })}
  >
    {@render body()}
  </button>
{:else}
  <div
    class="row"
    class:finished
    data-quest={row.id}
    data-status={row.status}
    data-worth={row.worth}
    data-change={phase === 'idle' ? undefined : phase}
    style:--row-out-ms={`${ROW_STEP_OUT_MS}ms`}
    style:--row-in-ms={`${ROW_STEP_IN_MS}ms`}
  >
    {@render body()}
  </div>
{/if}

<style>
  .row {
    position: relative;
    display: grid;
    grid-template-columns: var(--size-icon-sm) minmax(0, 1fr) auto var(--size-icon-sm);
    align-content: center;
    align-items: start;
    column-gap: var(--space-section-gap);
    box-sizing: border-box;
    width: 100%;
    min-height: var(--size-row-height);
    padding: calc(var(--space-unit) * 2) var(--space-row-pad);
    border: 0;
    border-radius: var(--radius-row);
    background: transparent;
    color: var(--color-text-primary);
    font-family: var(--font-family);
    text-align: left;
  }

  button.row {
    cursor: pointer;
  }

  button.row:hover {
    box-shadow: inset 0 0 0 var(--border-width) var(--color-hover-outline);
  }

  button.row:focus-visible {
    box-shadow: inset 0 0 0 var(--border-width) var(--color-edit-accent);
    outline: none;
  }

  /* The marks are 16 on a 20 line. */
  .row > :global(.status-mark),
  .row > :global(.worth-mark) {
    margin-top: 2px;
  }

  .title {
    display: flex;
    align-items: center;
    gap: var(--space-unit);
    min-width: 0;
  }

  .name {
    min-width: 0;
    overflow: hidden;
    font-size: var(--size-row);
    font-weight: var(--font-weight-row);
    line-height: var(--line-row);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* The "probably done" mark sits 4 px after the status mark: the column gap less the title's own gap. */
  .title > :global(.inferred-mark) {
    margin-left: calc(var(--space-unit) - var(--space-section-gap));
  }

  .manual {
    display: inline-flex;
    flex: none;
    color: var(--color-text-muted);
  }

  .rewards {
    max-width: 132px;
    overflow: hidden;
    color: var(--color-text-secondary);
    font-size: var(--size-label);
    font-weight: var(--font-weight-row);
    line-height: var(--line-row);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .step {
    grid-column: 2 / 4;
    overflow: hidden;
    color: var(--color-text-primary);
    font-size: var(--size-step);
    font-weight: 400;
    line-height: var(--line-step);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* In progress: the active band, the bar 3 px inside the left edge (so it shows inside the edit border)
     and 4 px in from the top and bottom, so two in-progress rows in a row show two bars, not one. */
  .row[data-status='in_progress'] {
    min-height: var(--size-row-height-tall);
    padding-bottom: 10px;
    background: var(--color-row-active-bg);
  }

  .row[data-status='in_progress']::before {
    content: '';
    position: absolute;
    top: 4px;
    bottom: 4px;
    left: 3px;
    width: var(--bar-in-progress);
    background: var(--color-status-in-progress);
  }

  /* Not worth it, still to do: the name steps back. */
  .row[data-worth='no']:not(.finished) .name {
    color: var(--color-text-secondary);
  }

  /* Finished: 32 high, name 14, every text and marker muted. */
  .row.finished {
    min-height: var(--size-row-height-finished);
    padding-top: 6px;
    padding-bottom: 6px;
  }

  .row.finished .name {
    color: var(--color-text-muted);
    font-size: var(--size-step);
  }

  .row.finished .rewards,
  .row.finished > :global(.worth-mark) {
    color: var(--color-text-muted);
  }

  .row[data-status='skipped'] .name {
    text-decoration: line-through;
  }

  /* The row transition: opacity and a 5 px rise only, on a strong ease-out. */
  .row {
    --row-ease: cubic-bezier(0.23, 1, 0.32, 1);
  }

  .leaving {
    animation: row-step-out var(--row-out-ms) var(--row-ease) forwards;
  }

  .entering {
    animation: row-step-in var(--row-in-ms) var(--row-ease) both;
  }

  @keyframes row-step-out {
    to {
      opacity: 0;
    }
  }

  @keyframes row-step-in {
    from {
      opacity: 0.2;
      transform: translateY(5px);
    }
  }

  /* Windows animations off: the change is instant (the script plays no phase either). */
  @media (prefers-reduced-motion: reduce) {
    .leaving,
    .entering {
      animation: none;
    }
  }
</style>
