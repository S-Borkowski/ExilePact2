<script lang="ts">
  // The panel header, 36 high. Play mode: the character, class and level, and
  // the view in use, with no control at all. Edit mode: the accent-tinted bar
  // with the EDIT chip, the character name that opens the picker, the view
  // switch, and the collapse and settings buttons. With no character the name
  // reads "Quest tracker", muted, with no picker and no switch. The name is
  // the panel's h1 in every variant, styled as before. A new character from
  // The Riverbank that no level-up has named yet (`provisional`) reads "New
  // character", level 1.
  import type { Send } from '../bridge';
  import type { View } from '../ipc';
  import { NEW_CHARACTER } from '../labels';
  import Icon from './Icon.svelte';

  let { view, send }: { view: View; send: Send } = $props();

  const edit = $derived(view.mode === 'edit');
  const character = $derived(view.character);
  const who = $derived(character?.provisional ? NEW_CHARACTER : (character?.name ?? ''));
  const meta = $derived(character ? `${character.class || 'Level'} ${character.level}${character.pinned ? ' · pinned' : ''}` : '');
  const kind = $derived(
    view.view_kind === 'next_up' ? 'Next up' : view.acts.length === 1 ? `All · ${view.acts[0].title}` : 'All',
  );
</script>

<header class="header" class:edit>
  {#if edit}
    <span class="chip"><Icon id="ui-edit" size="12px" /><span class="mode-tag">Edit</span></span>
    {#if character}
      <h1 class="title">
        <button
          type="button"
          class="who"
          title="Characters"
          aria-label={`${who}: choose the character`}
          onclick={() => send('open_frame', { frame: 'character-picker' })}
        >
          <span class="name" class:provisional={character.provisional}>{who}</span>
          <Icon id="ui-chevron-down" size="12px" />
        </button>
      </h1>
    {:else}
      <h1 class="name none">Quest tracker</h1>
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
      <h1 class="name" class:provisional={character.provisional}>{who}</h1>
      <span class="meta">{meta}</span>
      <span class="kind">{kind}</span>
    {:else}
      <h1 class="name none">Quest tracker</h1>
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

  /* The h1 around the picker button: no look of its own. */
  .title {
    display: flex;
    min-width: 0;
    margin: 0;
    font: inherit;
  }

  .name {
    min-width: 0;
    margin: 0;
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

  /* Muted is 4.3:1 on the edit tint, under AA for 16 px text (finish review F5). */
  .header.edit .name.none {
    color: var(--color-text-secondary);
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
    /* A 24 px target (finish review F14); transparent, so it looks the same. */
    min-height: 24px;
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
    isolation: isolate;
    padding: 1px;
    border: var(--border-width) solid var(--color-panel-border);
    background: var(--color-panel-sunken);
  }

  /*
   * Each button is a 24 px target (finish review F14) that still looks 20 px
   * high: 2 px of padding above and below, taken back by a negative margin so
   * the switch keeps its size, and the pressed face drawn 2 px in by ::before.
   */
  .switch button {
    position: relative;
    min-height: 24px;
    margin: -2px 0;
    padding: 2px var(--space-section-gap);
    border: 0;
    background: transparent;
    color: var(--color-text-muted);
    font-family: var(--font-family);
    font-size: var(--size-caption);
    font-weight: var(--font-weight-row);
    cursor: pointer;
  }

  .switch button[aria-pressed='true'] {
    color: var(--color-text-primary);
  }

  .switch button[aria-pressed='true']::before {
    position: absolute;
    z-index: -1;
    inset: 2px 0;
    background: var(--color-panel-raised);
    box-shadow: inset 0 0 0 var(--border-width) var(--color-panel-border);
    content: '';
  }

  .switch button:hover,
  .switch button:focus-visible {
    color: var(--color-text-primary);
    outline: none;
  }
</style>
