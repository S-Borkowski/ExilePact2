<script lang="ts">
  // The "worth it" marker: solid gold star, outline star or dash.
  import type { Worth } from '../ipc';
  import { WORTH_ICON, WORTH_LABEL } from '../labels';
  import Icon from './Icon.svelte';

  let { worth }: { worth: Worth } = $props();
</script>

<span class="worth-mark" data-worth={worth}>
  <Icon id={WORTH_ICON[worth]} label={WORTH_LABEL[worth]} size="16px" />
</span>

<style>
  .worth-mark {
    display: inline-flex;
    flex: none;
  }
  [data-worth='yes'] { color: var(--color-worth-gold); }
  [data-worth='optional'] { color: var(--color-text-secondary); }
  [data-worth='no'] { color: var(--color-worth-none); }
</style>
