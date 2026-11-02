<script lang="ts">
  // The Appearance section (`ui-design.md` § "Theme"): one select, the look
  // every window draws. Exile, the steel/masonry style, is the default;
  // Claude is the app's earlier look. It saves itself with one
  // `update_settings` key, as the Updates switch does, and shows what the
  // next `ch:app-settings` carries; a view without a theme is Exile.
  import type { Theme } from '../ipc';
  import type { SectionProps } from './sections';

  let { view, send }: SectionProps = $props();

  const THEMES: ReadonlyArray<readonly [Theme, string]> = [
    ['exile', 'Exile'],
    ['claude', 'Claude'],
  ];
</script>

<label class="row theme-row">
  <span class="label">Theme</span>
  <select
    class="field"
    value={view.theme ?? 'exile'}
    onchange={(e) => send('update_settings', { patch: { theme: e.currentTarget.value as Theme } })}
  >
    {#each THEMES as [value, name] (value)}
      <option {value} selected={value === (view.theme ?? 'exile')}>{name}</option>
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
    padding-bottom: calc(var(--space-unit) * 2);
  }

  .label {
    color: var(--color-text-primary);
    font-size: var(--size-label);
    font-weight: 500;
    line-height: var(--line-caption);
  }

  .field {
    min-width: 152px;
  }
</style>
