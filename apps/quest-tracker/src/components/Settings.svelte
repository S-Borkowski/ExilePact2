<script lang="ts">
  // The settings frame: one 40-high row per setting, label left and control
  // right, helper text under the opacity and click-through rows. Each control
  // sends `update_settings` with the one field it changes and shows what the
  // next `qt:view` says; the UI keeps no copy of its own. Play mode lists the
  // values with no control.
  //
  // After Live 2 (owner): opacity runs from 0 (no background; text, icons,
  // the border and the edit accent stay opaque), scale is an Auto switch plus
  // a slider for a fixed factor (moving it turns Auto off), and the dock side
  // is gone (the window is dragged by its header in edit mode).
  import type { Send } from '../bridge';
  import { type Scale, type View, SCALE_MAX, SCALE_MIN, SCALE_STEP } from '../ipc';
  import Icon from './Icon.svelte';

  let { view, send }: { view: View; send: Send } = $props();

  const s = $derived(view.settings);
  const edit = $derived(view.mode === 'edit');
  /** The factor the slider rests on while Auto is on, and where turning Auto off starts. */
  const AUTO_REST = 1;

  const percent = (v: number) => `${Math.round(v * 100)}%`;
  const factorText = (f: number) => `${Number(f.toFixed(2))}×`;
  const scaleText = (sc: Scale) => (sc === 'auto' ? 'Auto' : factorText(sc));
  const auto = $derived(s.scale === 'auto');
  const factor = $derived(s.scale === 'auto' ? AUTO_REST : s.scale);
  const opacityFill = $derived(percent(s.opacity));
  const scaleFill = $derived(percent((factor - SCALE_MIN) / (SCALE_MAX - SCALE_MIN)));
</script>

<div class="settings">
  <div class="frame-head">
    <h2 class="frame-title">Settings</h2>
    {#if edit}
      <button type="button" class="icon-btn" aria-label="Close" title="Close" onclick={() => send('open_frame', { frame: null })}>
        <Icon id="ui-close" size="16px" />
      </button>
    {/if}
  </div>

  {#if edit}
    <label class="row">
      <span class="label">Background opacity</span>
      <input
        type="range"
        class="slider"
        min="0"
        max="1"
        step="0.05"
        value={s.opacity}
        aria-valuetext={percent(s.opacity)}
        style:--fill={opacityFill}
        onchange={(e) => send('update_settings', { patch: { opacity: Number(e.currentTarget.value) } })}
      />
      <span class="value">{percent(s.opacity)}</span>
    </label>
    <p class="help">Text, icons and the panel's border always stay fully opaque.</p>

    <label class="row">
      <span class="label">Click-through (play mode)</span>
      <input type="checkbox" class="toggle" role="switch" checked={false} onchange={() => send('set_mode', { mode: 'play' })} />
    </label>
    <p class="help">On: in play mode the panel passes clicks to the game.</p>

    <label class="row">
      <span class="label">Auto scale</span>
      <input
        type="checkbox"
        class="toggle"
        role="switch"
        checked={auto}
        onchange={() => send('update_settings', { patch: { scale: auto ? AUTO_REST : 'auto' } })}
      />
    </label>
    <label class="row">
      <span class="label">Scale</span>
      <input
        type="range"
        class="slider"
        class:resting={auto}
        min={SCALE_MIN}
        max={SCALE_MAX}
        step={SCALE_STEP}
        value={factor}
        aria-valuetext={scaleText(s.scale)}
        style:--fill={scaleFill}
        onchange={(e) => send('update_settings', { patch: { scale: Number(e.currentTarget.value) } })}
      />
      <span class="value">{scaleText(s.scale)}</span>
    </label>
    <p class="help">Moving the slider sets a fixed scale and turns Auto off.</p>
    <label class="row">
      <span class="label">Show skipped quests</span>
      <input
        type="checkbox"
        class="toggle"
        role="switch"
        checked={s.show_skipped}
        onchange={() => send('update_settings', { patch: { show_skipped: !s.show_skipped } })}
      />
    </label>
    <label class="row">
      <span class="label">Only worth-it quests</span>
      <input
        type="checkbox"
        class="toggle"
        role="switch"
        checked={s.only_worth_it}
        onchange={() => send('update_settings', { patch: { only_worth_it: !s.only_worth_it } })}
      />
    </label>

    <div class="divider"></div>
    <label class="row">
      <span class="label">Edit-mode hotkey</span>
      <input
        type="text"
        class="text-input hotkey"
        value={s.hotkey}
        spellcheck="false"
        onchange={(e) => send('update_settings', { patch: { hotkey: e.currentTarget.value.trim() } })}
      />
    </label>

    <div class="divider"></div>
    <label class="field">
      <span class="label">Log file</span>
      <input
        type="text"
        class="text-input path-input"
        value={s.log_path ?? ''}
        title={s.log_path ?? undefined}
        placeholder="Client.txt path"
        spellcheck="false"
        onchange={(e) => send('update_settings', { patch: { log_path: e.currentTarget.value.trim() || null } })}
      />
    </label>
    <p class="help">Empty: found automatically.</p>
  {:else}
    <dl class="values">
      <div class="row"><dt>Background opacity</dt><dd>{percent(s.opacity)}</dd></div>
      <div class="row"><dt>Scale</dt><dd>{scaleText(s.scale)}</dd></div>
      <div class="row"><dt>Show skipped quests</dt><dd>{s.show_skipped ? 'On' : 'Off'}</dd></div>
      <div class="row"><dt>Only worth-it quests</dt><dd>{s.only_worth_it ? 'On' : 'Off'}</dd></div>
      <div class="row"><dt>Edit-mode hotkey</dt><dd><span class="keycap">{s.hotkey}</span></dd></div>
      <div class="field"><dt>Log file</dt><dd class="path" title={s.log_path ?? undefined}>{s.log_path ?? 'found automatically'}</dd></div>
    </dl>
  {/if}
</div>

<style>
  .settings {
    display: flex;
    flex-direction: column;
  }

  .frame-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-height: 40px;
    padding: 0 var(--space-section-gap) 0 var(--space-row-pad);
  }

  /* An `h2` for assistive technology; it looks like the span it replaced. */
  .frame-title {
    margin: 0;
    font-size: var(--size-title);
    font-weight: var(--font-weight-title);
    line-height: var(--line-title);
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-row-pad);
    box-sizing: border-box;
    min-height: 40px;
    padding: 0 var(--space-row-pad);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: var(--space-section-gap) var(--space-row-pad) 0;
  }

  .label,
  dt {
    color: var(--color-text-primary);
    font-size: var(--size-label);
    font-weight: var(--font-weight-row);
    line-height: var(--line-caption);
  }

  .value {
    min-width: 32px;
    color: var(--color-text-secondary);
    font-size: var(--size-caption);
    text-align: right;
    font-variant-numeric: tabular-nums;
  }

  .help {
    margin: -6px 0 0;
    padding: 0 var(--space-row-pad) 6px;
    color: var(--color-text-muted);
    font-size: var(--size-caption);
    line-height: var(--line-caption);
    letter-spacing: 0.2px;
  }

  /* Under a stacked field the help sits 4 px below the input, not on it. */
  .field + .help {
    margin-top: 4px;
  }

  /* A long path ends in an ellipsis; the whole path is in `title`. */
  .path-input {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .divider {
    height: var(--border-width);
    margin: var(--space-unit) 0;
    background: var(--color-panel-divider);
  }

  /* Auto on: the slider rests at 1× and its thumb steps back, until moved. */
  .slider.resting::-webkit-slider-thumb {
    background: var(--color-text-muted);
  }

  .hotkey {
    width: 120px;
    height: var(--size-control);
    text-align: center;
  }

  .values {
    margin: 0;
  }

  dd {
    margin: 0;
    color: var(--color-text-secondary);
    font-size: var(--size-label);
    line-height: var(--line-caption);
  }

  .path {
    overflow-wrap: anywhere;
  }
</style>
