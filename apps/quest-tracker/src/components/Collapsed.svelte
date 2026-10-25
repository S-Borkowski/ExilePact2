<script lang="ts">
  // The collapsed badge: the count of worth-it quests available now. In edit
  // mode it expands the panel.
  import type { Send } from '../bridge';
  import type { View } from '../ipc';
  import Icon from './Icon.svelte';

  let { view, send }: { view: View; send: Send } = $props();

  const count = $derived(view.available_worth_count);
  const label = $derived(`${count} worth-it ${count === 1 ? 'quest' : 'quests'} available`);
</script>

{#snippet face()}
  <Icon id="worth-yes" size="16px" />
  <span class="count">{count}</span>
{/snippet}

{#if view.mode === 'edit'}
  <button type="button" class="badge" class:has-new={count > 0} aria-label={`${label}; expand`} title="Expand" onclick={() => send('set_collapsed', { collapsed: false })}>
    {@render face()}
  </button>
{:else}
  <div class="badge" class:has-new={count > 0} role="status" aria-label={label}>{@render face()}</div>
{/if}

<style>
  .badge {
    display: inline-flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    width: 48px;
    height: 48px;
    box-sizing: border-box;
    padding: 0;
    border: var(--border-width) solid var(--color-panel-border);
    border-radius: var(--radius);
    background: color-mix(in srgb, var(--color-panel-bg) calc(var(--panel-opacity, 0.9) * 100%), transparent);
    box-shadow: var(--shadow-panel);
    color: var(--color-text-muted);
    font: inherit;
  }

  .badge.has-new {
    color: var(--color-worth-gold);
  }

  button.badge {
    cursor: pointer;
    border-color: var(--color-edit-accent);
  }

  .count {
    font-size: var(--size-title);
    font-weight: var(--font-weight-title);
    font-variant-numeric: tabular-nums;
    color: var(--color-text-primary);
  }
</style>
