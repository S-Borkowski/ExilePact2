<script lang="ts">
  // The Appearance section: one select, "Theme", Exile (the steel/masonry
  // kit, the default) or Claude (the look before it). It sends
  // `update_settings` with the Updates switch's argument shape and shows
  // what the next `wr:app-settings` says; the section keeps no copy of its
  // own. A view without a theme (an older shell) shows Exile.
  import type { Theme } from '../ipc';
  import { themeOf } from '../theme';
  import type { SectionProps } from './sections';

  let { view, send }: SectionProps = $props();

  const OPTIONS: Array<{ value: Theme; label: string }> = [
    { value: 'exile', label: 'Exile' },
    { value: 'claude', label: 'Claude' },
  ];

  function choose(event: Event & { currentTarget: HTMLSelectElement }): void {
    const theme = themeOf(event.currentTarget.value);
    // The select shows the view's theme until the next view answers.
    event.currentTarget.value = themeOf(view.theme);
    send('update_settings', { patch: { theme } });
  }
</script>

<label class="row theme-row">
  <span class="label">Theme</span>
  <select class="field theme-select" value={themeOf(view.theme)} onchange={choose}>
    {#each OPTIONS as option (option.value)}
      <option value={option.value} selected={option.value === themeOf(view.theme)}>{option.label}</option>
    {/each}
  </select>
</label>

<style>
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-row-pad);
    min-height: 40px;
    padding-bottom: var(--space-section-gap);
  }

  .label {
    color: var(--color-text-primary);
    font-size: var(--size-label);
    font-weight: 500;
    line-height: var(--line-caption);
  }

  .theme-select {
    width: 150px;
  }
</style>
