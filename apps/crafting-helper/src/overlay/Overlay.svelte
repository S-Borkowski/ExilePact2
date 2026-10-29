<script lang="ts">
  // The overlay's root (`overlay.html`, `ui-design.md` § "Overlay"): the
  // guided craft's current step over the game, from `view.overlay.guide` in
  // the last `View` the bridge delivered. At most seven one-line rows in
  // three groups: the status word, step and try; the instruction; the
  // uses and odds (or why it is stuck or must be planned again) and both
  // branch lines; the last outcome with its source; the screen watch's line.
  // The blueprint's name stays in the view for Rust's own lines and is
  // never drawn: the route is the app's recipe, not the player's. The
  // outcome's word is the page's; Rust's summary carries none.
  // Every string is Rust's but the labels; a row Rust sends nothing for is
  // left out, which is how stuck, replan and done drop theirs. The panel is
  // anchored to the window's edge the view's position names. The window is
  // click-through and never takes focus, and the shell alone shows or hides
  // it (`view.overlay.visible`), so the page draws what the view holds
  // either way and sends no command.
  import { onDestroy, onMount } from 'svelte';
  import type { Bridge } from '../bridge';
  import type { View } from '../ipc';
  import { OUTCOME_LABELS, SOURCE_LABELS, STATUS_LABELS } from '../craft/words';

  let { bridge }: { bridge: Bridge } = $props();

  let view = $state<View | null>(null);
  let stop: (() => void) | null = null;

  const guide = $derived(view?.overlay.guide ?? null);

  onMount(() => {
    let live = true;
    bridge
      .start((next) => {
        view = next;
      })
      .then(
        (unsubscribe) => {
          if (live) stop = unsubscribe;
          else unsubscribe();
        },
        (error) => console.error('crafting-helper overlay: the bridge did not start', error),
      );
    return () => {
      live = false;
    };
  });

  onDestroy(() => {
    stop?.();
  });
</script>

<div class="overlay" data-visible={view ? String(view.overlay.visible) : 'none'} data-position={view?.overlay.position ?? 'top-center'}>
  {#if guide}
    {@const g = guide}
    {@const middle = g.why !== null || g.uses.length > 0 || g.odds !== null || g.on_hit !== null || g.on_miss !== null}
    <div class="panel" data-status={g.status}>
      <div class="group">
        <p class="row head">
          <span class="status-word caps" data-status={g.status}>{STATUS_LABELS[g.status]}</span>
          {#if g.step_number !== null}
            <span class="step">Step {g.step_number}{#if g.try_number !== null}&nbsp;· try {g.try_number}{/if}</span>
          {/if}
        </p>
        <p class="row instruction">{g.instruction}</p>
      </div>

      {#if middle}
        <div class="group">
          {#if g.why !== null}
            <p class="row why"><span class="label">Why:</span> <span class="reason">{g.why}</span></p>
          {:else if g.uses.length > 0 || g.odds !== null}
            <p class="row uses">
              <span class="label caps">Uses</span>
              <span class="names">{g.uses.join(', ')}</span>{#if g.odds !== null}<span class="sep" aria-hidden="true">·</span><span class="odds">{g.odds}</span>{/if}
            </p>
          {/if}
          {#if g.on_hit !== null}<p class="row branch" data-branch="hit">{g.on_hit}</p>{/if}
          {#if g.on_miss !== null}<p class="row branch" data-branch="miss">{g.on_miss}</p>{/if}
        </div>
      {/if}

      <div class="group">
        {#if g.last}
          <p class="row last">
            <span class="label">Last:</span>
            <span class="result caps" data-result={g.last.result}>{OUTCOME_LABELS[g.last.result]}</span>
            <span class="source">{SOURCE_LABELS[g.last.source]}</span>
            <span class="summary">{g.last.summary}</span>
          </p>
        {:else}
          <p class="row last empty">No step taken yet.</p>
        {/if}
        {#if g.watch}
          <p class="row watch" data-warning={String(g.watch.warning)}>
            <span class="label caps">Screen</span>
            <span class="watch-text">{g.watch.text}</span>
          </p>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  /* The window: the panel at the edge nearest the game's edge, the rest
     empty and click-through. The window's own place (the 12 px inset) is
     Rust's. */
  .overlay {
    display: flex;
    flex-direction: column;
    justify-content: flex-start;
    height: 100%;
  }

  .overlay[data-position^='bottom'] {
    justify-content: flex-end;
  }

  /* The tracker's overlay ground: the theme's panel at 0.92, a 1 px edge,
     square, no blur, no shadow. The sides take the row padding (12 px), so
     no line starts at the panel's edge over a busy scene (audit, finding 1). */
  .panel {
    display: flex;
    flex-direction: column;
    gap: var(--space-unit);
    box-sizing: border-box;
    width: 100%;
    padding: var(--space-panel-pad) var(--space-row-pad);
    border: var(--border-width) solid var(--color-panel-border);
    background: color-mix(in srgb, var(--color-panel-bg) calc(var(--opacity-default) * 100%), transparent);
    color: var(--color-text-primary);
    font-size: var(--size-step);
    line-height: var(--line-step);
    font-variant-numeric: tabular-nums;
  }

  .group {
    display: flex;
    flex-direction: column;
  }

  /* One line each, cut at the end: the panel never grows. */
  .row {
    margin: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .head {
    display: flex;
    align-items: baseline;
    gap: calc(var(--space-unit) * 2);
    line-height: var(--line-title);
  }

  .status-word {
    flex: none;
    padding: 0 calc(var(--space-unit) * 1.5);
    border: var(--border-width) solid currentColor;
    color: var(--color-status-in-progress);
    font-size: var(--size-caption);
    line-height: var(--line-caption);
  }

  .status-word[data-status='stuck'],
  .status-word[data-status='replan'] {
    color: var(--error);
  }

  .status-word[data-status='done'] {
    color: var(--want);
  }

  .step {
    flex: none;
    color: var(--color-text-secondary);
  }

  .instruction {
    font-size: var(--size-title);
    line-height: var(--line-title);
    font-weight: var(--font-weight-title);
  }

  .label {
    color: var(--color-text-muted);
  }

  .label.caps {
    font-size: var(--size-caption);
  }

  .names {
    color: var(--color-text-primary);
  }

  .sep {
    margin: 0 calc(var(--space-unit) * 1.5);
    color: var(--color-text-muted);
  }

  .odds,
  .reason,
  .summary {
    color: var(--color-text-secondary);
  }

  .result {
    padding: 0 var(--space-unit);
    border: var(--border-width) solid currentColor;
    color: var(--want);
    font-size: var(--size-caption);
    line-height: var(--line-caption);
  }

  .result[data-result='miss'],
  .result[data-result='unexpected'] {
    color: var(--error);
  }

  .source,
  .last.empty {
    color: var(--color-text-muted);
  }

  .watch {
    color: var(--color-text-secondary);
  }

  .watch[data-warning='true'] .watch-text {
    color: var(--error);
  }
</style>
