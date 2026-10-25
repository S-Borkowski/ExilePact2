<script lang="ts">
  // The characters the log named: name over class and level, the active one
  // on the selected band with the accent bar and "Tracking". In edit mode:
  // select one, pin one (it then gets every event), or confirm an unconfirmed
  // one as the player's own.
  import type { Send } from '../bridge';
  import type { View } from '../ipc';
  import Icon from './Icon.svelte';

  let { view, send }: { view: View; send: Send } = $props();

  const edit = $derived(view.mode === 'edit');
</script>

<div class="picker">
  <div class="frame-head">
    <span class="frame-title">Characters</span>
    {#if edit}
      <button type="button" class="icon-btn" aria-label="Close" title="Close" onclick={() => send('open_frame', { frame: null })}>
        <Icon id="ui-close" size="16px" />
      </button>
    {/if}
  </div>
  {#if view.characters.length === 0}
    <p class="note">No character yet. Enter an area in game and it appears here.</p>
  {:else}
    <ul>
      {#each view.characters as c (c.name)}
        <li class="character" class:active={c.active} aria-current={c.active ? 'true' : undefined}>
          <span class="mark"><Icon id="ui-character" size="20px" /></span>
          {#snippet who()}
            <span class="name">{c.name}</span>
            <span class="meta">
              {c.class} · Level {c.level}{c.pinned ? ' · pinned' : ''}{c.own ? '' : ' · unconfirmed'}
            </span>
          {/snippet}
          {#if edit && !c.active}
            <button type="button" class="who" title="Track this character" onclick={() => send('select_character', { name: c.name })}>
              {@render who()}
            </button>
          {:else}
            <span class="who">{@render who()}</span>
          {/if}
          {#if c.active}
            <span class="tracking"><Icon id="ui-check" size="14px" /> Tracking</span>
          {/if}
          {#if edit}
            <span class="actions">
              <button type="button" class="small" onclick={() => send('pin_character', { name: c.pinned ? null : c.name })}>{c.pinned ? 'Unpin' : 'Pin'}</button>
              {#if !c.own}
                <button type="button" class="small" onclick={() => send('confirm_character', { name: c.name })}>Mine</button>
              {/if}
            </span>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
  <p class="note">Detected from Client.txt when you enter a zone.</p>
</div>

<style>
  .picker {
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

  ul {
    display: flex;
    flex-direction: column;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .character {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--space-row-pad);
    box-sizing: border-box;
    min-height: 48px;
    padding: 0 var(--space-row-pad);
  }

  .character.active {
    background: var(--color-select-active-bg);
  }

  .character.active::before {
    content: '';
    position: absolute;
    top: 0;
    bottom: 0;
    left: 0;
    width: var(--border-width-strong);
    background: var(--color-edit-accent);
  }

  .mark {
    display: inline-flex;
    color: var(--color-text-secondary);
  }

  .active .mark {
    color: var(--color-edit-accent);
  }

  .who {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
    padding: 0;
    border: 0;
    background: transparent;
    color: inherit;
    font: inherit;
    text-align: left;
  }

  button.who {
    cursor: pointer;
  }

  button.who:hover .name,
  button.who:focus-visible .name {
    text-decoration: underline;
    text-underline-offset: 3px;
  }

  button.who:focus-visible {
    outline: none;
  }

  .name {
    overflow: hidden;
    font-size: var(--size-row);
    font-weight: var(--font-weight-row);
    line-height: var(--line-row);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .meta,
  .tracking,
  .note {
    font-size: var(--size-caption);
    font-weight: var(--font-weight-row);
    line-height: var(--line-caption);
    letter-spacing: 0.2px;
  }

  .meta {
    color: var(--color-text-muted);
  }

  .tracking {
    display: inline-flex;
    flex: none;
    align-items: center;
    gap: var(--space-unit);
    color: var(--color-edit-accent);
  }

  .note {
    margin: 0;
    padding: var(--space-section-gap) var(--space-row-pad) 0;
    color: var(--color-text-muted);
  }

  .actions {
    display: flex;
    flex: none;
    gap: var(--space-unit);
  }

  .small {
    height: 22px;
    padding: 0 var(--space-section-gap);
    border: var(--border-width) solid var(--color-panel-border);
    background: var(--color-action-secondary-bg);
    color: var(--color-text-primary);
    font-family: var(--font-family);
    font-size: var(--size-caption);
    font-weight: var(--font-weight-title);
    cursor: pointer;
  }

  .small:hover {
    background: var(--color-action-secondary-hover);
  }

  .small:focus-visible {
    border-color: var(--color-edit-accent);
    outline: none;
  }
</style>
