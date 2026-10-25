<script lang="ts">
  // The four empty frames. Edit mode offers the one control that helps;
  // play mode says which hotkey reaches it.
  import type { Send } from '../bridge';
  import type { View } from '../ipc';

  let { view, send }: { view: View; send: Send } = $props();

  const edit = $derived(view.mode === 'edit');
  const actTitle = $derived(view.acts[0]?.title ?? 'this act');

  const COPY: Record<string, { title: string; text: string }> = $derived({
    'empty-no-log': { title: 'No log found', text: 'Client.txt was not found. Set its path in Settings.' },
    'empty-waiting': { title: 'Waiting for the game', text: 'The log is there; nothing new has been written to it yet.' },
    'empty-no-character': { title: 'No character yet', text: 'Enter an area in game, and the tracker picks up your character.' },
    'empty-act-done': { title: `${actTitle} done`, text: `Every quest in ${actTitle} is done or skipped.` },
  });
  const copy = $derived(COPY[view.frame] ?? { title: '', text: '' });
</script>

<div class="empty" data-empty={view.frame}>
  <p class="title">{copy.title}</p>
  <p class="text">{copy.text}</p>
  {#if edit}
    {#if view.frame === 'empty-no-log'}
      <button type="button" onclick={() => send('open_frame', { frame: 'settings' })}>Open settings</button>
    {:else if view.frame === 'empty-no-character'}
      <button type="button" onclick={() => send('open_frame', { frame: 'character-picker' })}>Characters</button>
    {:else if view.frame === 'empty-act-done'}
      <button type="button" onclick={() => send('set_view_kind', { kind: 'campaign' })}>Show the act</button>
    {/if}
  {:else if view.frame !== 'empty-waiting'}
    <p class="hint">Press {view.settings.hotkey} to edit.</p>
  {/if}
</div>

<style>
  .empty {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-unit);
    padding: var(--space-section-gap) var(--space-row-pad);
  }

  p {
    margin: 0;
  }

  .title {
    font-weight: var(--font-weight-title);
  }

  .text,
  .hint {
    color: var(--color-text-secondary);
    font-size: var(--size-step);
  }

  .hint {
    color: var(--color-text-muted);
    font-size: var(--size-caption);
  }

  button {
    padding: 4px var(--space-section-gap);
    border: var(--border-width) solid var(--color-edit-accent);
    border-radius: var(--radius-button);
    background: transparent;
    color: var(--color-text-primary);
    font: inherit;
    font-size: var(--size-caption);
    cursor: pointer;
  }
</style>
