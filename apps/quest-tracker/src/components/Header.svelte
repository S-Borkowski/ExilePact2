<script lang="ts">
  // The panel header, 36 high. Play mode: the character, class and level, and
  // the view in use, with no control at all. Edit mode: the accent-tinted bar
  // with the EDIT chip, the character name that opens the picker, the view
  // switch, and the collapse and settings buttons. With no character the name
  // reads "Quest tracker", muted, with no picker and no switch.
  import type { Send } from '../bridge';
  import type { View } from '../ipc';
  import Icon from './Icon.svelte';

  let { view, send }: { view: View; send: Send } = $props();

  const edit = $derived(view.mode === 'edit');
  const character = $derived(view.character);
  const kind = $derived(
    view.view_kind === 'next_up' ? 'Next up' : view.acts.length === 1 ? `All · ${view.acts[0].title}` : 'All',
  );
</script>

<header class="header" class:edit>
  {#if edit}
    <span class="chip"><Icon id="ui-edit" size="12px" /><span class="mode-tag">Edit</span></span>
    {#if character}
      <button
        type="button"
        class="who"
        title="Characters"
        aria-label={`${character.name}: choose the character`}
        onclick={() => send('open_frame', { frame: 'character-picker' })}
      >
        <span class="name">{character.name}</span>
        <Icon id="ui-chevron-down" size="12px" />
      </button>
    {:else}
      <span class="name none">Quest tracker</span>
    {/if}
    <span class="spacer"></span>
    {#if character}
      <div class="switch" role="group" aria-label="View">
        <button type="button" aria-pressed={view.view_kind === 'next_up'} onclick={() => send('set_view_kind', { kind: 'next_up' })}>Next up</button>
        <button type="button" aria-pressed={view.view_kind === 'campaign'} onclick={() => send('set_view_kind', { kind: 'campaign' })}>All</button>
      </div>
    {/if}
    <button type="button" class="icon-btn" title="Collapse" aria-label="Collapse" onclick={() => send('set_collapsed', { collapsed: true })}>
      <Icon id="ui-collapse" size="16px" />
    </button>
    <button type="button" class="icon-btn" title="Settings" aria-label="Settings" onclick={() => send('open_frame', { frame: 'settings' })}>
      <Icon id="ui-settings" size="16px" />
    </button>
  {:else}
    <span class="mark"><Icon id="ui-character" size="16px" /></span>
    {#if character}
      <span class="name">{character.name}</span>
      <span class="meta">{character.class} {character.level}{character.pinned ? ' · pinned' : ''}</span>
      <span class="kind">{kind}</span>
    {:else}
      <span class="name none">Quest tracker</span>
    {/if}
  {/if}
</header>

<style>
  .header {
    display: flex;
    flex: none;
    align-items: center;
    gap: var(--space-section-gap);
    box-sizing: border-box;
    min-height: var(--size-header);
    padding: 0 var(--space-row-pad);
    background: var(--gradient-header);
  }

  .header.edit {
    gap: 6px;
    background: var(--color-edit-tint);
  }

  .mark {
    display: inline-flex;
    color: var(--color-text-secondary);
  }

  .name {
    min-width: 0;
    overflow: hidden;
    color: var(--color-text-primary);
    font-size: var(--size-title);
    font-weight: var(--font-weight-title);
    line-height: var(--line-title);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .name.none {
    color: var(--color-text-muted);
  }

  .meta,
  .kind {
    font-size: var(--size-caption);
    line-height: var(--line-caption);
    letter-spacing: 0.2px;
    white-space: nowrap;
  }

  .meta {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    color: var(--color-text-muted);
    text-overflow: ellipsis;
  }

  .kind {
    color: var(--color-text-secondary);
  }

  .chip {
    display: inline-flex;
    flex: none;
    align-items: center;
    gap: 3px;
    padding: 2px 6px 2px 5px;
    background: var(--color-edit-accent);
    color: var(--color-panel-sunken);
  }

  .mode-tag {
    font-size: var(--size-caption);
    font-weight: 800;
    line-height: var(--line-caption);
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }

  .who {
    display: inline-flex;
    align-items: center;
    gap: var(--space-unit);
    min-width: 0;
    max-width: 104px;
    padding: 0;
    border: 0;
    background: transparent;
    color: var(--color-text-secondary);
    font: inherit;
    cursor: pointer;
  }

  .who:hover .name,
  .who:focus-visible .name {
    text-decoration: underline;
    text-underline-offset: 3px;
  }

  .who:focus-visible {
    outline: none;
  }

  .spacer {
    flex: 1;
  }

  .switch {
    display: flex;
    flex: none;
    padding: 1px;
    border: var(--border-width) solid var(--color-panel-border);
    background: var(--color-panel-sunken);
  }

  .switch button {
    height: 20px;
    padding: 0 var(--space-section-gap);
    border: 0;
    background: transparent;
    color: var(--color-text-muted);
    font-family: var(--font-family);
    font-size: var(--size-caption);
    font-weight: var(--font-weight-row);
    cursor: pointer;
  }

  .switch button[aria-pressed='true'] {
    background: var(--color-panel-raised);
    box-shadow: inset 0 0 0 var(--border-width) var(--color-panel-border);
    color: var(--color-text-primary);
  }

  .switch button:hover,
  .switch button:focus-visible {
    color: var(--color-text-primary);
    outline: none;
  }
</style>
