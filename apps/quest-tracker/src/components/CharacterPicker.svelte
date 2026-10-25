<script lang="ts">
  // The characters the log named: name, class, level, the active one marked.
  // In edit mode: select one, pin one (it then gets every event), or confirm
  // an unconfirmed one as the player's own.
  import type { Send } from '../bridge';
  import type { View } from '../ipc';
  import Icon from './Icon.svelte';

  let { view, send }: { view: View; send: Send } = $props();

  const edit = $derived(view.mode === 'edit');
</script>

<div class="picker">
  <div class="head">
    <span class="title">Characters</span>
    <span class="spacer"></span>
    {#if edit}
      <button type="button" class="icon-button" aria-label="Close" title="Close" onclick={() => send('open_frame', { frame: null })}>
        <Icon id="ui-close" />
      </button>
    {/if}
  </div>
  {#if view.characters.length === 0}
    <p class="note">No character yet. Enter an area in game and it appears here.</p>
  {:else}
    <ul>
      {#each view.characters as c (c.name)}
        <li class="character" class:active={c.active} aria-current={c.active ? 'true' : undefined}>
          <Icon id="ui-character" size="16px" />
          <span class="who">
            <span class="name">{c.name}</span>
            <span class="meta">
              {c.class} {c.level}{c.active ? ' · active' : ''}{c.pinned ? ' · pinned' : ''}{c.own ? '' : ' · unconfirmed'}
            </span>
          </span>
          {#if edit}
            <span class="actions">
              {#if !c.active}
                <button type="button" onclick={() => send('select_character', { name: c.name })}>Select</button>
              {/if}
              <button type="button" onclick={() => send('pin_character', { name: c.pinned ? null : c.name })}>{c.pinned ? 'Unpin' : 'Pin'}</button>
              {#if !c.own}
                <button type="button" onclick={() => send('confirm_character', { name: c.name })}>Mine</button>
              {/if}
            </span>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
  <p class="note">A new character appears here once the log names it.</p>
</div>

<style>
  .picker {
    display: flex;
    flex-direction: column;
    gap: var(--space-section-gap);
    padding: var(--space-unit) var(--space-row-pad);
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

  ul {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .character {
    display: flex;
    align-items: center;
    gap: var(--space-unit);
    padding: var(--space-unit);
    border-left: 2px solid transparent;
  }

  .character.active {
    border-left-color: var(--color-edit-accent);
    background: rgba(255, 255, 255, 0.05);
  }

  .who {
    display: flex;
    flex: 1;
    flex-direction: column;
  }

  .meta,
  .note {
    color: var(--color-text-muted);
    font-size: var(--size-caption);
  }

  .note {
    margin: 0;
  }

  .actions {
    display: flex;
    gap: 2px;
  }

  button {
    padding: 2px var(--space-unit);
    border: var(--border-width) solid var(--color-panel-border);
    border-radius: var(--radius-button);
    background: transparent;
    color: var(--color-text-primary);
    font: inherit;
    font-size: var(--size-caption);
    cursor: pointer;
  }

  .icon-button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 24px;
    height: 24px;
    padding: 0;
    border: 0;
  }
</style>
