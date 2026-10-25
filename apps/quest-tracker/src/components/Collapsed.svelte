<script lang="ts">
  // The collapsed badge, 48 × 48 with 4 px cut corners. Has new: a gold
  // border, a star and the count of worth-it quests available now. Idle: the
  // plain border and the expand glyph. Warning (the status line warns or
  // errs): the warning border and glyph. In edit mode it expands the panel,
  // and its border is the edit accent.
  import type { Send } from '../bridge';
  import type { View } from '../ipc';
  import Icon from './Icon.svelte';

  let { view, send }: { view: View; send: Send } = $props();

  const count = $derived(view.available_worth_count);
  const warning = $derived(view.status?.kind === 'warn' || view.status?.kind === 'error');
  const state = $derived(warning ? 'warning' : count > 0 ? 'has-new' : 'idle');
  const label = $derived(
    `${count} worth-it ${count === 1 ? 'quest' : 'quests'} available${warning && view.status ? `; ${view.status.text}` : ''}`,
  );
</script>

{#snippet face()}
  {#if state === 'warning'}
    <Icon id="ui-warning" size="20px" />
  {:else if state === 'has-new'}
    <Icon id="worth-yes" size="14px" />
  {:else}
    <Icon id="ui-expand" size="20px" />
  {/if}
  <span class="count" class:sr-only={state !== 'has-new'}>{count}</span>
{/snippet}

<div class="badge-shadow">
  {#if view.mode === 'edit'}
    <button type="button" class="badge cut edit" data-state={state} aria-label={`${label}; expand`} title="Expand" onclick={() => send('set_collapsed', { collapsed: false })}>
      {@render face()}
    </button>
  {:else}
    <div class="badge cut" data-state={state} role="status" aria-label={label}>{@render face()}</div>
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
