<script lang="ts">
  import type { Theme } from '../ipc';
  import type { SectionProps } from './sections';
  let { view, send }: SectionProps = $props();
</script>

<label class="theme-row">
  <span>Theme</span>
  <select class="select" value={view.theme ?? 'exile'} onchange={(event) => send('update_settings', { patch: { theme: event.currentTarget.value as Theme } })}>
    <option value="exile">Exile</option>
    <option value="claude">Claude</option>
  </select>
</label>

<style>
  .theme-row { display: flex; align-items: center; justify-content: space-between; gap: 12px; min-height: 40px; padding-bottom: 8px; }
</style>
