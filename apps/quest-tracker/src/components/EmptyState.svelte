<script lang="ts">
  // The four empty frames: a 24 px glyph, a title, a line of body text and
  // their captions. Edit mode offers the one control that helps; play mode
  // says which hotkey reaches it, on a keycap.
  import type { Send } from '../bridge';
  import type { View } from '../ipc';
  import Icon from './Icon.svelte';

  let { view, send }: { view: View; send: Send } = $props();

  const edit = $derived(view.mode === 'edit');
  const actTitle = $derived(view.acts[0]?.title ?? 'This act');

  const COPY: Record<string, { icon: string; title: string; text: string; hint: string }> = $derived({
    'empty-no-log': {
      icon: 'ui-warning',
      title: "Can't find Client.txt",
      text: 'The overlay reads quest progress from the game log. Point it at Client.txt in Settings, or start the game once so the file exists.',
      hint: 'then Settings › Log file',
    },
    'empty-waiting': {
      icon: 'status-in-progress',
      title: 'Waiting for the game',
      text: 'Client.txt is open but nothing new is in it yet. Log in with a character and your quests appear here.',
      hint: 'then Settings › Log file',
    },
    'empty-no-character': {
      icon: 'ui-character',
      title: 'No character yet',
      text: 'Enter any zone and the character is picked up from the log.',
      hint: 'then pick a character',
    },
    'empty-act-done': {
      icon: 'status-done',
      title: `${actTitle} complete`,
      text: `Every quest in ${actTitle} is done or skipped.`,
      hint: 'to show the act',
    },
  });
  const copy = $derived(COPY[view.frame] ?? { icon: 'status-unknown', title: '', text: '', hint: '' });
</script>

<div class="empty" data-empty={view.frame}>
  <span class="glyph" data-icon={copy.icon}><Icon id={copy.icon} size="24px" /></span>
  <p class="title">{copy.title}</p>
  <p class="text">{copy.text}</p>
  {#if view.frame === 'empty-no-log'}
    {#if view.settings.log_path}<p class="caption path">Looked in: {view.settings.log_path}</p>{/if}
    <p class="caption">Your manual ticks are kept.</p>
  {/if}
  {#if edit}
    <div class="actions">
      {#if view.frame === 'empty-no-log' || view.frame === 'empty-waiting'}
        <button type="button" class="btn cut primary" onclick={() => send('open_frame', { frame: 'settings' })}>Open settings</button>
      {:else if view.frame === 'empty-no-character'}
        <button type="button" class="btn cut primary" onclick={() => send('open_frame', { frame: 'character-picker' })}>Characters</button>
      {:else if view.frame === 'empty-act-done'}
        <button type="button" class="btn cut primary" onclick={() => send('set_view_kind', { kind: 'campaign' })}>Show the act</button>
      {/if}
    </div>
  {:else}
    <p class="hint">Press <span class="keycap">{view.settings.hotkey}</span> {copy.hint}</p>
  {/if}
</div>

<style>
  .empty {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-section-gap);
    padding: var(--space-row-pad) 16px 16px;
  }

  p {
    margin: 0;
  }

  .glyph {
    display: inline-flex;
    color: var(--color-text-secondary);
  }

  .glyph[data-icon='ui-warning'] {
    color: var(--color-warning);
  }

  .glyph[data-icon='status-done'] {
    color: var(--color-status-done);
  }

  .title {
    font-size: var(--size-title);
    font-weight: var(--font-weight-title);
    line-height: var(--line-title);
  }

  .text {
    color: var(--color-text-secondary);
    font-size: var(--size-step);
    font-weight: 400;
    line-height: var(--line-step);
  }

  .path {
    overflow-wrap: anywhere;
  }

  .actions {
    display: flex;
    gap: var(--space-section-gap);
    margin-top: var(--space-unit);
  }

  .hint {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    margin-top: var(--space-unit);
    color: var(--color-text-secondary);
    font-size: var(--size-label);
    line-height: var(--line-caption);
  }
</style>
