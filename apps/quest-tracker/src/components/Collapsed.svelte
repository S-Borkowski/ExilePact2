<script lang="ts">
  // The collapsed badge, 48 × 48 with 4 px cut corners. Has new: a gold
  // border, a star and the count of worth-it quests available now. Idle: the
  // plain border and the expand glyph. Warning (the status line warns or
  // errs): the warning border and glyph. In edit mode it expands the panel,
  // and its border is the edit accent.
  //
  // A live change of any quest (`qt:row-change`) highlights the badge as it
  // would the row (owner, after Live 2): the colour of what changed comes on
  // at once, holds `ROW_HIGHLIGHT_MS` and fades out; a second change restarts
  // the hold. Changes from before the badge was shown are not played.
  import { onDestroy, untrack } from 'svelte';
  import type { Send } from '../bridge';
  import type { RowChangeKind, View } from '../ipc';
  import { ROW_HIGHLIGHT_MS, ROW_HIGHLIGHT_OUT_MS, getRowChanges } from '../rowchange';
  import Icon from './Icon.svelte';

  let { view, send }: { view: View; send: Send } = $props();

  const changes = getRowChanges();
  let played = changes?.latest ?? 0;
  let highlight = $state<RowChangeKind | null>(null);
  let timer: ReturnType<typeof setTimeout> | undefined;

  $effect(() => {
    const change = changes?.newest();
    if (!change || change.id <= played) return;
    played = change.id;
    untrack(() => {
      clearTimeout(timer);
      highlight = change.kind;
      timer = setTimeout(() => (highlight = null), ROW_HIGHLIGHT_MS);
    });
  });

  onDestroy(() => clearTimeout(timer));

  const count = $derived(view.available_worth_count);
  const warning = $derived(view.status?.kind === 'warn' || view.status?.kind === 'error');
  const look = $derived(warning ? 'warning' : count > 0 ? 'has-new' : 'idle');
  const label = $derived(
    `${count} worth-it ${count === 1 ? 'quest' : 'quests'} available${warning && view.status ? `; ${view.status.text}` : ''}`,
  );
</script>

{#snippet face()}
  {#if look === 'warning'}
    <Icon id="ui-warning" size="20px" />
  {:else if look === 'has-new'}
    <Icon id="worth-yes" size="14px" />
  {:else}
    <Icon id="ui-expand" size="20px" />
  {/if}
  <span class="count" class:sr-only={look !== 'has-new'}>{count}</span>
{/snippet}

<div class="badge-shadow">
  {#if view.mode === 'edit'}
    <button
      type="button"
      class="badge cut edit"
      data-state={look}
      data-highlight={highlight ?? undefined}
      style:--badge-highlight-out-ms={`${ROW_HIGHLIGHT_OUT_MS}ms`}
      aria-label={`${label}; expand`}
      title="Expand"
      onclick={() => send('set_collapsed', { collapsed: false })}
    >
      {@render face()}
    </button>
  {:else}
    <div
      class="badge cut"
      data-state={look}
      data-highlight={highlight ?? undefined}
      style:--badge-highlight-out-ms={`${ROW_HIGHLIGHT_OUT_MS}ms`}
      role="status"
      aria-label={label}
    >
      {@render face()}
    </div>
  {/if}
</div>

<style>
  .badge-shadow {
    filter: var(--drop-panel);
  }

  .badge {
    --cut: var(--chamfer-badge);
    --ring-color: var(--color-panel-border);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 2px;
    box-sizing: border-box;
    width: var(--size-badge);
    height: var(--size-badge);
    padding: 0;
    border: 0;
    border-radius: var(--radius);
    background: color-mix(in srgb, var(--color-panel-bg) calc(var(--panel-opacity, var(--opacity-default)) * 100%), transparent);
    color: var(--color-text-secondary);
    font: inherit;
  }

  .badge[data-state='has-new'] {
    --ring: var(--border-width-strong);
    --ring-color: var(--color-worth-gold);
    color: var(--color-worth-gold);
  }

  .badge[data-state='warning'] {
    --ring-color: var(--color-warning);
    color: var(--color-warning);
  }

  button.badge {
    cursor: pointer;
  }

  /*
   * The highlight: the changed quest's status colour mixed into the face,
   * on at once, faded back by colour after the hold. Colour only, so reduced
   * motion keeps it.
   */
  .badge {
    transition: background-color var(--badge-highlight-out-ms) cubic-bezier(0.23, 1, 0.32, 1);
  }

  .badge[data-highlight] {
    --badge-tint: var(--color-status-in-progress);
    background-color: color-mix(
      in srgb,
      var(--badge-tint) 20%,
      color-mix(in srgb, var(--color-panel-bg) calc(var(--panel-opacity, var(--opacity-default)) * 100%), transparent)
    );
    transition: none;
  }

  .badge[data-highlight='available'] {
    --badge-tint: var(--color-status-available);
  }

  .badge[data-highlight='quest_done'] {
    --badge-tint: var(--color-status-done);
  }

  .badge.edit {
    --ring: var(--border-width-strong);
    --ring-color: var(--color-edit-accent);
  }

  .count {
    font-size: var(--size-badge-count);
    font-weight: 800;
    line-height: 24px;
    font-variant-numeric: tabular-nums;
  }
</style>
