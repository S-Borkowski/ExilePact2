<script lang="ts">
  // The toast stack, below the panel's bottom edge (never above it). The
  // Undo action exists only in edit mode: play mode shows no control.
  import type { ShownToast } from '../toasts';
  import Icon from './Icon.svelte';

  let { toasts, edit, onUndo }: { toasts: ShownToast[]; edit: boolean; onUndo: (toast: ShownToast) => void } = $props();
</script>

{#if toasts.length > 0}
  <ol class="toasts" aria-live="polite">
    {#each toasts as toast (toast.key)}
      <li class="toast" data-kind={toast.kind}>
        <Icon id={toast.icon} size="16px" />
        <span class="text">{toast.text}</span>
        {#if toast.undo && edit}
          <button type="button" onclick={() => onUndo(toast)}>Undo</button>
        {/if}
      </li>
    {/each}
  </ol>
{/if}

<style>
  .toasts {
    display: flex;
    flex-direction: column;
    gap: var(--space-unit);
    margin: var(--space-section-gap) 0 0;
    padding: 0;
    list-style: none;
  }

  .toast {
    display: flex;
    align-items: center;
    gap: var(--space-section-gap);
    box-sizing: border-box;
    width: 300px;
    max-width: 100%;
    min-height: 36px;
    padding: var(--space-unit) var(--space-row-pad);
    border-left: 3px solid var(--color-status-available);
    border-radius: var(--radius);
    background: var(--color-toast-bg);
    box-shadow: var(--shadow-toast);
    font-size: var(--size-step);
  }

  [data-kind='step_done'] {
    border-left-color: var(--color-status-in-progress);
  }

  [data-kind='quest_done'] {
    border-left-color: var(--color-status-done);
  }

  [data-kind='undo'] {
    border-left-color: var(--color-edit-accent);
  }

  [data-kind='available'] :global(.icon) {
    color: var(--color-status-available);
  }

  .text {
    flex: 1;
  }

  button {
    padding: 2px var(--space-unit);
    border: var(--border-width) solid var(--color-edit-accent);
    border-radius: var(--radius-button);
    background: transparent;
    color: var(--color-text-primary);
    font: inherit;
    font-size: var(--size-caption);
    cursor: pointer;
  }
</style>
