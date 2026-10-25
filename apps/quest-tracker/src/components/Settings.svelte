<script lang="ts">
  // The settings frame: one 40-high row per setting, label left and control
  // right, helper text under the opacity and click-through rows. Each control
  // sends `update_settings` with the one field it changes and shows what the
  // next `qt:view` says; the UI keeps no copy of its own. Play mode lists the
  // values with no control.
  import type { Send } from '../bridge';
  import { type Dock, type Scale, type View, DOCKS, SCALES } from '../ipc';
  import Icon from './Icon.svelte';

  let { view, send }: { view: View; send: Send } = $props();

  const s = $derived(view.settings);
  const edit = $derived(view.mode === 'edit');
  const MIN_OPACITY = 0.3;

  const SCALE_LABEL: Record<Scale, string> = { auto: 'Auto', '1': '1×', '1.33': '1.33×', '2': '2×' };
  const DOCK_LABEL: Record<Dock, string> = { left: 'Left', right: 'Right' };
  const percent = (v: number) => `${Math.round(v * 100)}%`;
  const fill = $derived(`${Math.round(((s.opacity - MIN_OPACITY) / (1 - MIN_OPACITY)) * 100)}%`);
</script>

<div class="settings">
  <div class="frame-head">
    <span class="frame-title">Settings</span>
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
        min={MIN_OPACITY}
        max="1"
        step="0.05"
        value={s.opacity}
        style:--fill={fill}
        onchange={(e) => send('update_settings', { patch: { opacity: Number(e.currentTarget.value) } })}
      />
      <span class="value">{percent(s.opacity)}</span>
    </label>
    <p class="help">Text and icons always stay fully opaque. Floor {percent(MIN_OPACITY)}.</p>

    <label class="row">
      <span class="label">Click-through (play mode)</span>
      <input type="checkbox" class="toggle" role="switch" checked={false} onchange={() => send('set_mode', { mode: 'play' })} />
    </label>
    <p class="help">On returns to play mode: the panel passes clicks to the game.</p>

    <label class="row">
      <span class="label">Dock side</span>
      <span class="select-wrap">
        <select class="select" value={s.dock} onchange={(e) => send('update_settings', { patch: { dock: e.currentTarget.value as Dock } })}>
          {#each DOCKS as d (d)}<option value={d}>{DOCK_LABEL[d]}</option>{/each}
        </select>
        <Icon id="ui-chevron-down" size="12px" />
      </span>
    </label>
    <label class="row">
      <span class="label">Scale</span>
      <span class="select-wrap">
        <select class="select" value={s.scale} onchange={(e) => send('update_settings', { patch: { scale: e.currentTarget.value as Scale } })}>
          {#each SCALES as sc (sc)}<option value={sc}>{SCALE_LABEL[sc]}</option>{/each}
        </select>
        <Icon id="ui-chevron-down" size="12px" />
      </span>
    </label>
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
        class="text-input"
        value={s.log_path ?? ''}
        placeholder="Client.txt path"
        spellcheck="false"
        onchange={(e) => send('update_settings', { patch: { log_path: e.currentTarget.value.trim() || null } })}
      />
    </label>
    <p class="help">Empty: found automatically.</p>
  {:else}
    <dl class="values">
      <div class="row"><dt>Background opacity</dt><dd>{percent(s.opacity)}</dd></div>
      <div class="row"><dt>Dock side</dt><dd>{DOCK_LABEL[s.dock]}</dd></div>
      <div class="row"><dt>Scale</dt><dd>{SCALE_LABEL[s.scale]}</dd></div>
      <div class="row"><dt>Show skipped quests</dt><dd>{s.show_skipped ? 'On' : 'Off'}</dd></div>
      <div class="row"><dt>Only worth-it quests</dt><dd>{s.only_worth_it ? 'On' : 'Off'}</dd></div>
      <div class="row"><dt>Edit-mode hotkey</dt><dd><span class="keycap">{s.hotkey}</span></dd></div>
      <div class="field"><dt>Log file</dt><dd class="path">{s.log_path ?? 'found automatically'}</dd></div>
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

  .frame-title {
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

  .divider {
    height: var(--border-width);
    margin: var(--space-unit) 0;
    background: var(--color-panel-divider);
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
