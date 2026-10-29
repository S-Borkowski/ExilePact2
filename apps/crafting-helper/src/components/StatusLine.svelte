<script lang="ts">
  // The main window's status line, its one polite live region: the app's
  // last message as sent. An error starts with "!" and takes the error
  // colour, so it is not told by colour alone.
  import type { StatusLine } from '../ipc';

  let { status }: { status: StatusLine | null } = $props();
</script>

<p class="status" role="status" aria-live="polite" data-kind={status?.kind ?? 'none'}>
  {#if status}
    {#if status.kind === 'error'}<span class="mark" aria-hidden="true">!</span>{/if}<span>{status.text}</span>
  {/if}
</p>

<style>
  /* A row, so a wrapped error hangs its lines after the "!". */
  .status {
    display: flex;
    align-items: baseline;
    min-height: calc(var(--line-step) + var(--space-unit) * 4);
    margin: 0;
    padding: calc(var(--space-unit) * 2) calc(var(--space-unit) * 5);
    border-top: var(--border-width) solid var(--color-panel-divider);
    color: var(--color-text-secondary);
    font-size: var(--size-step);
    line-height: var(--line-step);
  }

  .status[data-kind='error'] {
    color: var(--error);
  }

  .mark {
    flex: none;
    margin-right: calc(var(--space-unit) * 1.5);
    font-weight: 700;
  }
</style>
