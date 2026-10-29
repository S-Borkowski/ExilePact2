<script lang="ts">
  // The Overlay section (`ui-design.md` § "Settings window"): where the step
  // panel sits over the game, and whether the app reads the hovered item
  // from the screen. Each control saves itself with one `update_settings`
  // key and shows what the next `ch:app-settings` carries; the section keeps
  // no copy of its own. The select offers only the six places the app
  // takes.
  import type { OverlayPosition } from '../ipc';
  import type { SectionProps } from './sections';

  let { view, send }: SectionProps = $props();

  /** The six places in reading order, with their names. */
  const PLACES: ReadonlyArray<readonly [OverlayPosition, string]> = [
    ['top-left', 'Top left'],
    ['top-center', 'Top center'],
    ['top-right', 'Top right'],
    ['bottom-left', 'Bottom left'],
    ['bottom-center', 'Bottom center'],
    ['bottom-right', 'Bottom right'],
  ];
</script>

<div class="row">
  <label class="label" for="overlay-position">Position</label>
  <select
    id="overlay-position"
    class="field"
    aria-describedby="overlay-position-help"
    value={view.overlay_position}
    onchange={(e) => send('update_settings', { patch: { overlay_position: e.currentTarget.value as OverlayPosition } })}
  >
    {#each PLACES as [value, name] (value)}
      <option {value} selected={value === view.overlay_position}>{name}</option>
    {/each}
  </select>
</div>
<p class="help" id="overlay-position-help">Where the step panel sits in the game's window while a craft is guided.</p>

<label class="row switch-row">
  <span class="label">Read the item from the screen</span>
  <input
    type="checkbox"
    class="toggle"
    role="switch"
    checked={view.screen_watch}
    aria-describedby="screen-watch-help"
    onchange={(e) => send('update_settings', { patch: { screen_watch: e.currentTarget.checked } })}
  />
</label>
<p class="help last" id="screen-watch-help">
  After each step, hover the item in the game and the helper reads it. Copies of the screen stay in memory; nothing is saved or sent. Off:
  grab the item after each step.
</p>

<style>
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-row-pad);
    min-height: 40px;
  }

  .switch-row {
    cursor: pointer;
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

  .help {
    margin: -6px 0 var(--space-section-gap);
    color: var(--color-text-muted);
    font-size: var(--size-label);
    line-height: var(--line-step);
    letter-spacing: 0.2px;
  }

  .help.last {
    margin-bottom: 0;
    padding-bottom: calc(var(--space-unit) * 3);
  }
</style>
