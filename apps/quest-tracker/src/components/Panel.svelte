<script lang="ts">
  // The expanded panel: header, the status line, the body the view's frame
  // names, and in edit mode the way back to play mode. Cut corners (6 px), the
  // background at the opacity setting while text and icons stay opaque, a
  // 1 px border that edit mode turns into the 2 px accent. At opacity 0 the
  // background is gone and the border and accent still draw: they are the
  // ring (`.cut::after`), which the setting does not touch.
  import type { Send } from '../bridge';
  import type { QuestRow as Row, StatusKind, View, Worth } from '../ipc';
  import { WORTH_LABEL } from '../labels';
  import ActSection from './ActSection.svelte';
  import CharacterPicker from './CharacterPicker.svelte';
  import EmptyState from './EmptyState.svelte';
  import Header from './Header.svelte';
  import Icon from './Icon.svelte';
  import QuestDetail from './QuestDetail.svelte';
  import QuestRow from './QuestRow.svelte';
  import Settings from './Settings.svelte';

  let { view, send, drag = () => {} }: { view: View; send: Send; drag?: () => void } = $props();

  const edit = $derived(view.mode === 'edit');
  const STATUS_ICON = { ok: 'ui-check', info: null, warn: 'ui-warning', error: 'ui-warning' } as const;

  // Next up arrives sorted by worth (worth it, optional, not worth it). A
  // label goes above the first row of each run; the order stays the view's.
  function byWorth(rows: Row[]): Array<{ worth: Worth; rows: Row[] }> {
    const groups: Array<{ worth: Worth; rows: Row[] }> = [];
    for (const row of rows) {
      const last = groups[groups.length - 1];
      if (last && last.worth === row.worth) last.rows.push(row);
      else groups.push({ worth: row.worth, rows: [row] });
    }
    return groups;
  }

  const groups = $derived(byWorth(view.next_up));

  // The empty frames whose body already states the condition, and the status
  // kind the shell sends for it: the line over them would say it twice
  // (finish review F7). Any other kind there is another problem and shows.
  const STATED_BY_BODY: Partial<Record<View['frame'], StatusKind>> = {
    'empty-no-log': 'error',
    'empty-waiting': 'info',
  };
  const status = $derived(view.status && STATED_BY_BODY[view.frame] === view.status.kind ? null : view.status);
</script>

<div class="panel-shadow">
  <div class="panel cut" class:edit>
    <Header {view} {send} {drag} />
    {#if status}
      {@const icon = STATUS_ICON[status.kind]}
      <p class="status-line" role="status" data-kind={status.kind}>
        {#if icon}<Icon id={icon} size="14px" />{/if}
        <span>{status.text}</span>
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
          {#each groups as group, i (i)}
            <p class="group-label">{WORTH_LABEL[group.worth]}</p>
            {#each group.rows as row (row.id)}
              <QuestRow {row} {edit} {send} />
            {/each}
          {/each}
        {/if}
      {:else if view.frame === 'quest-detail' && view.detail}
        <QuestDetail detail={view.detail} {edit} {send} />
      {:else if view.frame === 'character-picker'}
        <CharacterPicker {view} {send} />
      {:else if view.frame === 'settings'}
        <Settings {view} {send} />
      {:else if view.frame.startsWith('empty-')}
        <EmptyState {view} {send} />
      {/if}
    </div>
    {#if edit}
      <button type="button" class="back-to-play" onclick={() => send('set_mode', { mode: 'play' })}>
        <Icon id="ui-click-through" size="12px" />
        <span><span class="key">{view.settings.hotkey}</span> returns to play mode (click-through)</span>
      </button>
    {/if}
  </div>
</div>

<style>
  .panel-shadow {
    filter: var(--drop-panel);
  }

  .panel {
    --cut: var(--chamfer-panel);
    --ring-color: var(--color-panel-border);
    display: flex;
    flex-direction: column;
    box-sizing: border-box;
    width: var(--size-panel-width);
    max-height: 60vh;
    border-radius: var(--radius);
    /* The opacity setting fades the background only; text and icons stay opaque. */
    background:
      var(--gradient-panel),
      color-mix(in srgb, var(--color-panel-bg) calc(var(--panel-opacity, var(--opacity-default)) * 100%), transparent);
    /* The 1 px highlight along the top edge (white at 5%). */
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.05);
    color: var(--color-text-primary);
  }

  .panel.edit {
    --ring: var(--border-width-strong);
    --ring-color: var(--color-edit-accent);
  }

  .status-line {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0;
    padding: var(--space-unit) var(--space-row-pad);
    border-bottom: var(--border-width) solid var(--color-panel-divider);
    color: var(--color-text-secondary);
    font-size: var(--size-caption);
    line-height: var(--line-caption);
  }

  .status-line[data-kind='ok'] :global(.icon) {
    color: var(--color-status-available);
  }

  .status-line[data-kind='warn'],
  .status-line[data-kind='error'] {
    color: var(--color-warning);
  }

  .body {
    overflow-y: auto;
    padding-bottom: var(--space-panel-pad);
    scrollbar-color: var(--color-panel-border) transparent;
    scrollbar-width: thin;
  }

  .note {
    margin: 0;
    padding: var(--space-section-gap) var(--space-row-pad);
    color: var(--color-text-muted);
    font-size: var(--size-step);
  }

  .back-to-play {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: var(--space-section-gap) var(--space-row-pad) 10px;
    border: 0;
    border-top: var(--border-width) solid var(--color-panel-divider);
    background: transparent;
    color: var(--color-edit-accent);
    font-family: var(--font-family);
    font-size: var(--size-caption);
    font-weight: var(--font-weight-row);
    line-height: var(--line-caption);
    letter-spacing: 0.2px;
    text-align: left;
    cursor: pointer;
  }

  .back-to-play:hover > span,
  .back-to-play:focus-visible > span {
    text-decoration: underline;
    text-underline-offset: 2px;
  }

  .back-to-play:focus-visible {
    outline: none;
  }

  .key {
    font-weight: var(--font-weight-title);
  }
</style>
