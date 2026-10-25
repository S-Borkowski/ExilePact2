<script lang="ts">
  // The expanded panel: header, the status line, and the body the view's
  // frame names. Edit mode tints the frame and the header with the accent.
  import type { Send } from '../bridge';
  import type { QuestAction, View } from '../ipc';
  import ActSection from './ActSection.svelte';
  import CharacterPicker from './CharacterPicker.svelte';
  import EmptyState from './EmptyState.svelte';
  import Header from './Header.svelte';
  import QuestDetail from './QuestDetail.svelte';
  import QuestRow from './QuestRow.svelte';
  import Settings from './Settings.svelte';

  let { view, send, onAction }: { view: View; send: Send; onAction: (questId: string, action: QuestAction) => void } = $props();

  const edit = $derived(view.mode === 'edit');
  const STATUS_GLYPH = { ok: '✓', info: 'i', warn: '!', error: '×' } as const;
</script>

<div class="panel" class:edit>
  <Header {view} {send} />
  {#if view.status}
    <p class="status-line" data-kind={view.status.kind}>
      <span class="status-glyph" aria-hidden="true">{STATUS_GLYPH[view.status.kind]}</span>
      {view.status.text}
    </p>
  {/if}
  <div class="body">
    {#if view.frame === 'panel-campaign'}
      {#each view.acts as act (act.act)}
        <ActSection {act} {edit} {send} />
      {/each}
    {:else if view.frame === 'panel-next-up'}
      {#if view.next_up.length === 0}
        <p class="note">Nothing available or in progress in this act.</p>
      {:else}
        {#each view.next_up as row (row.id)}
          <QuestRow {row} {edit} {send} />
        {/each}
      {/if}
    {:else if view.frame === 'quest-detail' && view.detail}
      {@const detail = view.detail}
      <QuestDetail {detail} {edit} {send} onAction={(action) => onAction(detail.row.id, action)} />
    {:else if view.frame === 'character-picker'}
      <CharacterPicker {view} {send} />
    {:else if view.frame === 'settings'}
      <Settings {view} {send} />
    {:else if view.frame.startsWith('empty-')}
      <EmptyState {view} {send} />
    {/if}
  </div>
</div>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    box-sizing: border-box;
    width: 320px;
    max-height: 60vh;
    overflow: hidden;
    border: var(--border-width) solid var(--color-panel-border);
    border-radius: var(--radius);
    /* The opacity setting fades the background only; text and icons stay opaque. */
    background:
      var(--gradient-panel),
      color-mix(in srgb, var(--color-panel-bg) calc(var(--panel-opacity, 0.9) * 100%), transparent);
    box-shadow: var(--shadow-panel);
    color: var(--color-text-primary);
  }

  .panel.edit {
    border-color: var(--color-edit-accent);
  }

  .panel.edit :global(.header) {
    background: color-mix(in srgb, var(--color-edit-accent) 18%, transparent);
  }

  .status-line {
    display: flex;
    align-items: center;
    gap: var(--space-unit);
    margin: 0;
    padding: 2px var(--space-row-pad);
    color: var(--color-text-secondary);
    font-size: var(--size-caption);
  }

  .status-line[data-kind='warn'],
  .status-line[data-kind='error'] {
    color: var(--color-res-fire);
  }

  .status-glyph {
    font-weight: var(--font-weight-title);
  }

  .body {
    overflow-y: auto;
    padding: var(--space-unit) 0 var(--space-panel-pad);
  }

  .note {
    margin: 0;
    padding: var(--space-unit) var(--space-row-pad);
    color: var(--color-text-muted);
    font-size: var(--size-step);
  }
</style>
