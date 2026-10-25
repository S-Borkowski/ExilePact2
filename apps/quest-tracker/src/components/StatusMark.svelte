<script lang="ts">
  // A quest's status: a shape (the status icon or glyph) and a colour, so it
  // reads without colour too.
  import type { QuestStatus } from '../ipc';
  import { STATUS_ICON, STATUS_LABEL } from '../labels';
  import Icon from './Icon.svelte';

  let { status }: { status: QuestStatus } = $props();
</script>

<span class="status-mark" data-status={status}>
  <Icon id={STATUS_ICON[status]} label={STATUS_LABEL[status]} size="16px" />
</span>

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
