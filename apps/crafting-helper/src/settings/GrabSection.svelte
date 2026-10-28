<script lang="ts">
  // The Grab section: the global hotkey that grabs an item after the game's
  // own Ctrl+Alt+C, a field to change it, and what the app said about the
  // last change. The app decides: it refuses Ctrl+Alt+C in any spelling, and
  // a key another program holds, and keeps the old hotkey; the section shows
  // the hotkey and the status the next `ch:app-settings` carries.
  import type { SectionProps } from './sections';

  let { view, send }: SectionProps = $props();

  let draft = $state('');
  const wanted = $derived(draft.trim());

  function submit(event: SubmitEvent): void {
    event.preventDefault();
    if (wanted === '') return;
    send('set_hotkey', { hotkey: wanted });
    draft = '';
  }
</script>

<p class="row current">
  <span class="label">Hotkey</span>
  <kbd class="hotkey">{view.hotkey}</kbd>
</p>
<p class="help" id="grab-help">
  Press Ctrl+Alt+C in the game to copy an item, then this hotkey to grab it. It cannot be Ctrl+Alt+C, the game's own copy.
</p>

<form class="set" onsubmit={submit}>
  <label class="visually-hidden" for="hotkey-field">New hotkey</label>
  <input
    id="hotkey-field"
    class="field"
    type="text"
    placeholder="e.g. Ctrl+Alt+G"
    autocomplete="off"
    spellcheck="false"
    maxlength="64"
    aria-describedby="grab-help hotkey-status"
    bind:value={draft}
  />
  <button type="submit" class="btn" disabled={wanted === ''}>Set</button>
</form>

<p class="status" id="hotkey-status" role="status" aria-live="polite" data-kind={view.hotkey_status?.kind ?? 'none'}>
  {#if view.hotkey_status}
    {#if view.hotkey_status.kind === 'error'}<span class="mark" aria-hidden="true">!</span>{/if}<span>{view.hotkey_status.text}</span>
  {/if}
</p>

<style>
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-row-pad);
    min-height: 40px;
    margin: 0;
  }

  .label {
    color: var(--color-text-primary);
    font-size: var(--size-label);
    font-weight: 500;
    line-height: var(--line-caption);
  }

  kbd {
    padding: 0 calc(var(--space-unit) * 1.5);
    border: var(--border-width) solid var(--color-panel-border);
    background: var(--color-panel-raised);
    color: var(--color-text-primary);
    font-family: var(--mono);
    font-size: var(--size-caption);
    line-height: var(--line-step);
    white-space: nowrap;
  }

  .help {
    margin: -6px 0 var(--space-section-gap);
    color: var(--color-text-muted);
    font-size: var(--size-label);
    line-height: var(--line-step);
    letter-spacing: 0.2px;
  }

  .set {
    display: flex;
    gap: var(--space-section-gap);
  }

  .set .field {
    flex: 1;
    min-width: 0;
    font-family: var(--mono);
    user-select: text;
  }

  /* A row, so a wrapped refusal hangs its lines after the "!". */
  .status {
    display: flex;
    align-items: baseline;
    min-height: var(--line-step);
    margin: 0;
    padding: var(--space-unit) 0 calc(var(--space-unit) * 3);
    color: var(--color-text-secondary);
    font-size: var(--size-step);
    line-height: var(--line-step);
  }

  .status[data-kind='error'] {
    color: var(--error);
  }

  .mark {
    flex: none;
    margin-right: calc(var(--space-unit) * 1.5);
    font-weight: 700;
  }
</style>
