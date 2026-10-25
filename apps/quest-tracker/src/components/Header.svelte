<script lang="ts">
  // The panel header: title, the character progress goes to, the view in
  // use, and in edit mode the controls and the "Edit" tag. Play mode shows
  // the same text with no control at all.
  import type { Send } from '../bridge';
  import type { View } from '../ipc';
  import Icon from './Icon.svelte';

  let { view, send }: { view: View; send: Send } = $props();

  const edit = $derived(view.mode === 'edit');
  const who = $derived(
    view.character
      ? `${view.character.name} · ${view.character.class} ${view.character.level}${view.character.pinned ? ' · pinned' : ''}`
      : 'No character',
  );
</script>

<header class="header">
  <div class="top">
    <span class="title">Quests</span>
    {#if edit}
      <span class="mode-tag">Edit</span>
      <span class="spacer"></span>
      <button type="button" class="icon-button" title="Characters" aria-label="Characters" onclick={() => send('open_frame', { frame: 'character-picker' })}>
        <Icon id="ui-character" />
      </button>
      <button type="button" class="icon-button" title="Settings" aria-label="Settings" onclick={() => send('open_frame', { frame: 'settings' })}>
        <Icon id="ui-settings" />
      </button>
      <button type="button" class="icon-button" title="Collapse" aria-label="Collapse" onclick={() => send('set_collapsed', { collapsed: true })}>
        <Icon id="ui-collapse" />
      </button>
      <button type="button" class="icon-button" title="Back to play mode (click-through)" aria-label="Play mode" onclick={() => send('set_mode', { mode: 'play' })}>
        <Icon id="ui-click-through" />
      </button>
    {:else}
      <span class="spacer"></span>
      <span class="kind">{view.view_kind === 'next_up' ? 'Next up' : 'Campaign'}</span>
    {/if}
  </div>
  <div class="character">{who}</div>
  {#if edit}
    <div class="switch" role="group" aria-label="View">
      <button type="button" aria-pressed={view.view_kind === 'next_up'} onclick={() => send('set_view_kind', { kind: 'next_up' })}>Next up</button>
      <button type="button" aria-pressed={view.view_kind === 'campaign'} onclick={() => send('set_view_kind', { kind: 'campaign' })}>Campaign</button>
    </div>
  {/if}
</header>

<style>
  .header {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: var(--space-unit) var(--space-row-pad);
    background: var(--gradient-header);
    border-bottom: var(--border-width) solid var(--color-panel-border);
  }

  .top {
    display: flex;
    align-items: center;
    gap: var(--space-unit);
  }

  .title {
    font-size: var(--size-title);
    font-weight: var(--font-weight-title);
  }

  .mode-tag {
    padding: 0 var(--space-unit);
    border: var(--border-width) solid var(--color-edit-accent);
    border-radius: var(--radius-button);
    color: var(--color-edit-accent);
    font-size: var(--size-caption);
    text-transform: uppercase;
    letter-spacing: 0.08em;
  }

  .spacer {
    flex: 1;
  }

  .kind,
  .character {
    font-size: var(--size-caption);
    color: var(--color-text-secondary);
  }

  .icon-button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    padding: 0;
    border: 0;
    border-radius: var(--radius-button);
    background: transparent;
    color: var(--color-text-secondary);
    cursor: pointer;
  }

  .icon-button:hover,
  .icon-button:focus-visible {
    color: var(--color-text-primary);
    background: rgba(255, 255, 255, 0.08);
    outline: none;
  }

  .switch {
    display: flex;
    gap: var(--space-unit);
  }

  .switch button {
    flex: 1;
    padding: 2px var(--space-unit);
    border: var(--border-width) solid var(--color-panel-border);
    border-radius: var(--radius-button);
    background: transparent;
    color: var(--color-text-secondary);
    font: inherit;
    font-size: var(--size-caption);
    cursor: pointer;
  }

  .switch button[aria-pressed='true'] {
    border-color: var(--color-edit-accent);
    color: var(--color-text-primary);
  }
</style>
