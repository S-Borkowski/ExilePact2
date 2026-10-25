<script lang="ts">
  // The toast stack, below the panel's bottom edge (never above it: the
  // minimap sits there), 4 px apart. Each toast: 300 × 44, 4 px cut corners,
  // a 3 px bar and a 20 px icon in its type's colour, one line of text. The
  // Undo action exists only in edit mode (play mode shows no control), with a
  // countdown bar over the 5 s it stays.
  import type { ShownToast } from '../toasts';
  import Icon from './Icon.svelte';

  let { toasts, edit, onUndo }: { toasts: ShownToast[]; edit: boolean; onUndo: (toast: ShownToast) => void } = $props();
</script>

{#if toasts.length > 0}
  <ol class="toasts" aria-live="polite">
    {#each toasts as toast (toast.key)}
      <li class="toast cut" data-kind={toast.kind}>
        <span class="mark"><Icon id={toast.icon} size="20px" /></span>
        <span class="text">{toast.text}</span>
        {#if toast.undo && edit}
          <button type="button" class="undo" onclick={() => onUndo(toast)}>Undo</button>
        {/if}
        {#if toast.undo}<span class="countdown" style:--countdown="var(--motion-undo-ms)"></span>{/if}
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
    filter: var(--drop-toast);
    list-style: none;
  }

  .toast {
    --cut: var(--chamfer-toast);
    --tone: var(--color-status-available);
    display: flex;
    align-items: center;
    gap: 10px;
    box-sizing: border-box;
    width: var(--size-toast-width);
    max-width: 100%;
    min-height: var(--size-toast-height);
    padding: var(--space-unit) var(--space-row-pad) var(--space-unit) calc(var(--bar-toast) + 9px);
    border-radius: var(--radius);
    background: var(--color-toast-bg);
    color: var(--color-text-primary);
    animation: toast-in var(--motion-duration) var(--motion-easing);
  }

  .toast::before {
    content: '';
    position: absolute;
    top: 0;
    bottom: 0;
    left: 0;
    width: var(--bar-toast);
    background: var(--tone);
  }

  [data-kind='step_done'] {
    --tone: var(--color-status-in-progress);
  }

  [data-kind='quest_done'] {
    --tone: var(--color-status-done);
  }

  [data-kind='undo'] {
    --tone: var(--color-edit-accent);
  }

  .mark {
    display: inline-flex;
    color: var(--tone);
  }

  .text {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    font-size: var(--size-row);
    font-weight: var(--font-weight-row);
    line-height: var(--line-row);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .undo {
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--color-edit-accent);
    font-family: var(--font-family);
    font-size: var(--size-label);
    font-weight: var(--font-weight-title);
    cursor: pointer;
  }

  .undo:hover,
  .undo:focus-visible {
    text-decoration: underline;
    text-underline-offset: 2px;
    outline: none;
  }

  .toast > :global(.countdown) {
    color: var(--tone);
  }

  @keyframes toast-in {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .toast {
      animation: none;
    }
  }
</style>
