<script lang="ts">
  // Exile distinguishes quest states by diamond fill and outline; Claude
  // keeps its original symbols. Both expose the same single status name.
  import type { QuestStatus } from '../ipc';
  import { STATUS_ICON, STATUS_LABEL } from '../labels';
  import Icon from './Icon.svelte';

  let { status, exile = false }: { status: QuestStatus; exile?: boolean } = $props();
</script>

{#if exile}
  <span class="status-mark diamond-mark" data-status={status} role="img" aria-label={STATUS_LABEL[status]} title={STATUS_LABEL[status]}>
    <span class="status-diamond" aria-hidden="true"></span>
  </span>
{:else}
  <span class="status-mark" data-status={status}>
    <Icon id={STATUS_ICON[status]} label={STATUS_LABEL[status]} size="16px" />
  </span>
{/if}

<style>
  .status-mark {
    display: inline-flex;
    flex: none;
  }
  [data-status='locked'] { color: var(--color-status-locked); }
  [data-status='available'] { color: var(--color-status-available); }
  [data-status='in_progress'] { color: var(--color-status-in-progress); }
  [data-status='done'] { color: var(--color-status-done); }
  [data-status='skipped'] { color: var(--color-status-skipped); }
  [data-status='unknown'] { color: var(--color-status-unknown); }
</style>
