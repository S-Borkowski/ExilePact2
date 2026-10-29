<script lang="ts">
  // The craft area (`ui-design.md` § "Craft window", "Layout"): one of the
  // craft part's four modes, picked from `craft.mode` alone. From 880 px of
  // window width planning and guiding are two columns that scroll on their
  // own, the Start or End bar at its column's foot; below, one column that
  // scrolls as a whole, the bar fixed at the window's foot. When the mode
  // changes, the focus moves to the new mode's first heading, so a keyboard
  // user is not left on a control that is gone.
  import { tick } from 'svelte';
  import type { Send } from '../bridge';
  import type { CatalogueGroup, CraftMode, View } from '../ipc';
  import GrabPanel from '../components/GrabPanel.svelte';
  import BlockedForCraft from './BlockedForCraft.svelte';
  import CurrentStep from './CurrentStep.svelte';
  import EndBar from './EndBar.svelte';
  import History from './History.svelte';
  import ItemPanel from './ItemPanel.svelte';
  import Refused from './Refused.svelte';
  import RoutePreview from './RoutePreview.svelte';
  import Targets from './Targets.svelte';

  let { view, catalogue, send }: { view: View; catalogue: CatalogueGroup[]; send: Send } = $props();

  const craft = $derived(view.craft);

  let area = $state<HTMLElement | null>(null);
  let shownMode: CraftMode | null = null;

  $effect(() => {
    const mode = craft.mode;
    if (shownMode !== null && mode !== shownMode) {
      void tick().then(() => area?.querySelector<HTMLElement>('[data-mode-heading]')?.focus());
    }
    shownMode = mode;
  });

  /** Why Start crafting is disabled, said beside it; null while it is enabled. */
  const startWhy = $derived.by(() => {
    const planning = craft.planning;
    if (!planning || planning.can_accept) return null;
    return planning.targets.length === 0 || planning.preview === null ? 'Pick a target first.' : 'No route to start.';
  });
</script>

<div class="craft" data-mode={craft.mode} bind:this={area}>
  {#if craft.mode === 'planning' && craft.planning}
    {@const planning = craft.planning}
    <div class="scroll">
      <div class="col left">
        <ItemPanel item={planning.item} grab={view.grab} modeHeading />
        <Targets {planning} {send} />
      </div>
      <div class="col right">
        <BlockedForCraft blocked={planning.blocked} preview={planning.preview} {catalogue} {send} />
        <RoutePreview preview={planning.preview} {send} />
      </div>
    </div>
    <div class="bar start-bar" data-under="right">
      <button type="button" class="btn primary" disabled={!planning.can_accept} onclick={() => send('accept_craft', {})}>Start crafting</button>
      {#if startWhy}<p class="why">{startWhy}</p>{/if}
    </div>
  {:else if craft.mode === 'guiding' && craft.guiding}
    {@const guiding = craft.guiding}
    <div class="scroll">
      <div class="col left">
        <CurrentStep {guiding} hotkey={view.hotkey} />
      </div>
      <div class="col right">
        <div class="item-now"><ItemPanel item={guiding.item} grab={view.grab} /></div>
        <History history={guiding.history} />
      </div>
    </div>
    <div class="bar" data-under="left">
      {#key guiding.status}
        <EndBar status={guiding.status} {send} />
      {/key}
    </div>
  {:else if craft.mode === 'refused'}
    <div class="scroll single">
      <Refused refusal={craft.refusal ?? ''} grab={view.grab} hotkey={view.hotkey} />
    </div>
  {:else}
    <div class="scroll single">
      <GrabPanel {view} />
    </div>
  {/if}
</div>

<style>
  /* One column below 880 px: the columns scroll as a whole, the bar stays at
     the window's foot above the status line. */
  .craft {
    display: grid;
    grid-template-rows: minmax(0, 1fr) auto;
    grid-template-areas:
      'scroll'
      'bar';
    min-height: 0;
  }

  .scroll {
    grid-area: scroll;
    min-height: 0;
    overflow-y: auto;
  }

  .single {
    grid-row: 1 / -1;
    padding: calc(var(--space-unit) * 4) calc(var(--space-unit) * 5);
  }

  .col {
    display: flex;
    flex-direction: column;
    gap: calc(var(--space-section-gap) * 2);
    padding: calc(var(--space-unit) * 4) calc(var(--space-unit) * 5);
    min-width: 0;
    min-height: 0;
  }

  .bar {
    grid-area: bar;
  }

  .start-bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: calc(var(--space-unit) * 3);
    padding: calc(var(--space-unit) * 3) calc(var(--space-unit) * 5);
    border-top: var(--border-width) solid var(--color-panel-divider);
    background: var(--color-panel-bg);
  }

  .why {
    margin: 0;
    color: var(--color-text-secondary);
    font-size: var(--size-step);
    line-height: var(--line-step);
  }

  /* From 880 px: two columns, about 45 % and 55 %, each scrolling on its
     own; the bar sits at the foot of the column it belongs to. */
  @media (min-width: 880px) {
    .craft[data-mode='planning'] {
      grid-template-columns: 45fr 55fr;
      grid-template-rows: minmax(0, 1fr) auto;
      grid-template-areas:
        'left right'
        'left bar';
    }

    .craft[data-mode='guiding'] {
      grid-template-columns: 45fr 55fr;
      grid-template-rows: minmax(0, 1fr) auto;
      grid-template-areas:
        'left right'
        'bar right';
    }

    .craft[data-mode='planning'] .scroll,
    .craft[data-mode='guiding'] .scroll {
      display: contents;
    }

    .left {
      grid-area: left;
      overflow-y: auto;
    }

    .right {
      grid-area: right;
      overflow-y: auto;
      border-left: var(--border-width) solid var(--color-panel-divider);
    }

    .craft[data-mode='planning'] .bar {
      border-left: var(--border-width) solid var(--color-panel-divider);
    }
  }
</style>
