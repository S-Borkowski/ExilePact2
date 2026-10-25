<script lang="ts">
  // The settings frame. Each control sends `update_settings` with the one
  // field it changes and shows what the next `qt:view` says; the UI keeps no
  // copy of its own. Play mode lists the values with no control.
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
</script>

<div class="settings">
  <div class="head">
    <span class="title">Settings</span>
    <span class="spacer"></span>
    {#if edit}
      <button type="button" class="icon-button" aria-label="Close" title="Close" onclick={() => send('open_frame', { frame: null })}>
        <Icon id="ui-close" />
      </button>
    {/if}
  </div>

  {#if edit}
    <label class="field">
      <span>Background opacity <span class="value">{percent(s.opacity)}</span></span>
      <input
        type="range"
        min={MIN_OPACITY}
        max="1"
        step="0.05"
        value={s.opacity}
        onchange={(e) => send('update_settings', { patch: { opacity: Number(e.currentTarget.value) } })}
      />
    </label>
    <label class="check">
      <input type="checkbox" checked={false} onchange={() => send('set_mode', { mode: 'play' })} />
      Click-through (back to play mode)
    </label>
    <label class="field">
      <span>Dock</span>
      <select value={s.dock} onchange={(e) => send('update_settings', { patch: { dock: e.currentTarget.value as Dock } })}>
        {#each DOCKS as d (d)}<option value={d}>{DOCK_LABEL[d]}</option>{/each}
      </select>
    </label>
    <label class="field">
      <span>Scale</span>
      <select value={s.scale} onchange={(e) => send('update_settings', { patch: { scale: e.currentTarget.value as Scale } })}>
        {#each SCALES as sc (sc)}<option value={sc}>{SCALE_LABEL[sc]}</option>{/each}
      </select>
    </label>
    <label class="check">
      <input type="checkbox" checked={s.show_skipped} onchange={() => send('update_settings', { patch: { show_skipped: !s.show_skipped } })} />
      Show skipped quests
    </label>
    <label class="check">
      <input type="checkbox" checked={s.only_worth_it} onchange={() => send('update_settings', { patch: { only_worth_it: !s.only_worth_it } })} />
      Only worth-it quests
    </label>
    <label class="field">
      <span>Edit-mode hotkey</span>
      <input
        type="text"
        value={s.hotkey}
        spellcheck="false"
        onchange={(e) => send('update_settings', { patch: { hotkey: e.currentTarget.value.trim() } })}
      />
    </label>
    <label class="field">
      <span>Log file (empty: find it)</span>
      <input
        type="text"
        value={s.log_path ?? ''}
        placeholder="Client.txt path"
        spellcheck="false"
        onchange={(e) => send('update_settings', { patch: { log_path: e.currentTarget.value.trim() || null } })}
      />
    </label>
  {:else}
    <dl class="values">
      <dt>Background opacity</dt><dd>{percent(s.opacity)}</dd>
      <dt>Dock</dt><dd>{DOCK_LABEL[s.dock]}</dd>
      <dt>Scale</dt><dd>{SCALE_LABEL[s.scale]}</dd>
      <dt>Show skipped</dt><dd>{s.show_skipped ? 'Yes' : 'No'}</dd>
      <dt>Only worth-it</dt><dd>{s.only_worth_it ? 'Yes' : 'No'}</dd>
      <dt>Edit-mode hotkey</dt><dd>{s.hotkey}</dd>
      <dt>Log file</dt><dd>{s.log_path ?? 'found automatically'}</dd>
    </dl>
  {/if}
</div>

<style>
  .settings {
    display: flex;
    flex-direction: column;
    gap: var(--space-section-gap);
    padding: var(--space-unit) var(--space-row-pad);
    font-size: var(--size-step);
  }

  .head {
    display: flex;
    align-items: center;
  }

  .title {
    font-weight: var(--font-weight-title);
  }

  .spacer {
    flex: 1;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .check {
    display: flex;
    align-items: center;
    gap: var(--space-unit);
  }

  .value,
  dt {
    color: var(--color-text-muted);
    font-size: var(--size-caption);
  }

  input[type='text'],
  select {
    padding: 2px var(--space-unit);
    border: var(--border-width) solid var(--color-panel-border);
    border-radius: var(--radius-button);
    background: rgba(0, 0, 0, 0.3);
    color: var(--color-text-primary);
    font: inherit;
    font-size: var(--size-caption);
  }

  .values {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 2px var(--space-section-gap);
    margin: 0;
  }

  dd {
    margin: 0;
    overflow-wrap: anywhere;
  }

  .icon-button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--color-text-secondary);
    cursor: pointer;
  }
</style>
