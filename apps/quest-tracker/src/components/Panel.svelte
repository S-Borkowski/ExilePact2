<script lang="ts">
  // The expanded panel: header, the status line, the body the view's frame
  // names, and in edit mode the way back to play mode. Cut corners (6 px), the
  // background at the opacity setting while text and icons stay opaque, a
  // 1 px border that edit mode turns into the 2 px accent. At opacity 0 the
  // background is gone and the border and accent still draw: they are the
  // ring (`.cut::after`), which the setting does not touch.
  //
  // The panel fills the window inside its shadow margin, in both directions,
  // and its body scrolls: the player sizes the window, the scale only zooms
  // its content (0.1.1). The margin stays 12 window pixels at every scale
  // (the overlay divides it by the zoom), so the frame keeps its size on
  // screen while the scale changes. In edit mode its four edges and four corners are resize
  // handles, bands that straddle the border with the matching cursor and
  // nothing drawn; a left press starts the window's resize from that edge
  // (`resize`, the bridge's window resize), as the header's press moves it.
  // Play mode has none: the window is click-through there.
  import { RESIZE_DIRECTIONS, type ResizeDirection, type Send } from '../bridge';
  import type { QuestRow as Row, StatusKind, View, Worth } from '../ipc';
  import { WORTH_LABEL } from '../labels';
  import ActSection from './ActSection.svelte';
  import CharacterPicker from './CharacterPicker.svelte';
  import EmptyState from './EmptyState.svelte';
  import Header from './Header.svelte';
  import Icon from './Icon.svelte';
  import MaterialFrame from './MaterialFrame.svelte';
  import crest from '../../theme/materials/raster/lantern-astrolabe-crest.png';
  import stone from '../../theme/materials/raster/charcoal-stone-well.png';
  import QuestDetail from './QuestDetail.svelte';
  import QuestRow from './QuestRow.svelte';
  import Settings from './Settings.svelte';

  let {
    view,
    send,
    drag = () => {},
    resize = () => {},
  }: { view: View; send: Send; drag?: () => void; resize?: (direction: ResizeDirection) => void } = $props();

  const edit = $derived(view.mode === 'edit');
  const journal = $derived(view.settings.theme !== 'claude' && edit && view.frame !== 'panel-next-up');
  const plaque = $derived(view.frame === 'settings' ? 'Overlay settings' : view.frame === 'character-picker' ? 'Characters' : view.frame === 'quest-detail' ? 'Quest details' : 'Campaign journal');
  let panelElement = $state<HTMLDivElement>();
  let wholeScroll = $state(false);

  // Text enlargement can make the fixed chrome taller than the window.
  // Keep all content reachable in one scroll area in that case. This affects
  // presentation only; native bounds and the received view never change.
  $effect(() => {
    view.frame; view.mode; view.status; view.settings.theme;
    if (!panelElement || typeof ResizeObserver === 'undefined') return;
    const panel = panelElement.querySelector<HTMLDivElement>(':scope > .panel-content') ?? panelElement;
    const measure = () => {
      const css = getComputedStyle(panel);
      const chrome = [...panel.children].filter((child) => !child.classList.contains('body'));
      const remaining = panel.clientHeight - parseFloat(css.paddingTop) - parseFloat(css.paddingBottom)
        - chrome.reduce((height, child) => height + child.getBoundingClientRect().height, 0);
      wholeScroll = remaining < 72;
    };
    const observer = new ResizeObserver(measure);
    observer.observe(panel);
    for (const child of panel.children) observer.observe(child);
    measure();
    return () => observer.disconnect();
  });

  function pressEdge(event: PointerEvent, direction: ResizeDirection): void {
    if (event.button !== 0) return;
    event.preventDefault();
    resize(direction);
  }

  function pressDragSurface(event: PointerEvent): void {
    if (!edit || event.button !== 0) return;
    if (event.target instanceof Element && event.target.closest('button, input, select, textarea, a, [role="button"]')) return;
    drag();
  }
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

{#snippet panelContents()}
    {#if journal}
      <!-- The native window gesture has no keyboard action; the controls remain buttons. -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="journal-plaque" aria-hidden="true" onpointerdown={pressDragSurface}>
        <img class="journal-crest" src={crest} alt="" draggable="false" />
        <span>{plaque}</span>
      </div>
    {/if}
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
{/snippet}

<div class="panel-shadow" data-material={journal ? 'journal' : 'compact'}>
  {#if journal}<MaterialFrame />{/if}
  {#if edit && journal}
    <!-- Fixed inside the top rail, outside the scroll well and resize bands. -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="frame-drag-region" aria-hidden="true" title="Drag to move Quest Tracker" onpointerdown={pressDragSurface}></div>
  {/if}
  <div bind:this={panelElement} class="panel cut" class:edit style:--well-image={journal ? `url("${stone}")` : null} data-scroll={wholeScroll ? 'whole' : 'body'} data-material={view.settings.theme === 'claude' ? 'claude' : journal ? 'journal' : 'compact'}>
    {#if view.settings.theme !== 'claude'}
      <div class="panel-content">{@render panelContents()}</div>
    {:else}
      {@render panelContents()}
    {/if}
  </div>
  {#if edit}
    {#each RESIZE_DIRECTIONS as direction (direction)}
      <!-- A mouse gesture on the window's own frame, which the keyboard has
           no use for: the window keeps its size otherwise. -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="resize-edge" data-resize={direction} aria-hidden="true" onpointerdown={(e) => pressEdge(e, direction)}></div>
    {/each}
  {/if}
</div>

<style>
  /* The window's height and width inside the overlay's margin, at every
     scale: the overlay is the viewport, which the webview zoom keeps equal
     to the window. */
  .panel-shadow {
    position: relative;
    display: flex;
    flex: 1 1 auto;
    align-self: stretch;
    min-height: 0;
    filter: var(--drop-panel);
  }

  .panel {
    --cut: var(--chamfer-panel);
    --ring-color: var(--color-panel-border);
    display: flex;
    flex: 1 1 auto;
    flex-direction: column;
    box-sizing: border-box;
    width: 100%;
    min-height: 0;
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
    flex: none;
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
    flex: 1 1 auto;
    min-height: 0;
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
    flex: none;
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

  /* Edit mode's resize handles: an 8 px band straddling each edge (half in
     the shadow margin, half over the border) and a 14 px square on each
     corner, above the edges. Nothing is drawn: the cursor is the affordance. */
  .resize-edge {
    --band: 8px;
    --corner: 14px;
    position: absolute;
    z-index: 1;
  }

  .resize-edge[data-resize='North'],
  .resize-edge[data-resize='South'] {
    right: calc(var(--corner) / 2);
    left: calc(var(--corner) / 2);
    height: var(--band);
    cursor: ns-resize;
  }

  .resize-edge[data-resize='East'],
  .resize-edge[data-resize='West'] {
    top: calc(var(--corner) / 2);
    bottom: calc(var(--corner) / 2);
    width: var(--band);
    cursor: ew-resize;
  }

  .resize-edge[data-resize='North'] {
    top: calc(var(--band) / -2);
  }

  .resize-edge[data-resize='South'] {
    bottom: calc(var(--band) / -2);
  }

  .resize-edge[data-resize='East'] {
    right: calc(var(--band) / -2);
  }

  .resize-edge[data-resize='West'] {
    left: calc(var(--band) / -2);
  }

  .resize-edge[data-resize='NorthWest'],
  .resize-edge[data-resize='NorthEast'],
  .resize-edge[data-resize='SouthEast'],
  .resize-edge[data-resize='SouthWest'] {
    z-index: 2;
    width: var(--corner);
    height: var(--corner);
  }

  .resize-edge[data-resize='NorthWest'] {
    top: calc(var(--corner) / -2);
    left: calc(var(--corner) / -2);
    cursor: nwse-resize;
  }

  .resize-edge[data-resize='SouthEast'] {
    right: calc(var(--corner) / -2);
    bottom: calc(var(--corner) / -2);
    cursor: nwse-resize;
  }

  .resize-edge[data-resize='NorthEast'] {
    top: calc(var(--corner) / -2);
    right: calc(var(--corner) / -2);
    cursor: nesw-resize;
  }

  .resize-edge[data-resize='SouthWest'] {
    bottom: calc(var(--corner) / -2);
    left: calc(var(--corner) / -2);
    cursor: nesw-resize;
  }
</style>
