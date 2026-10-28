<script lang="ts">
  // The mod list: a toolbar (text filter, "Chosen only", "Clear choices")
  // and the three groups. The filter and "Chosen only" are the page's own
  // and only hide rows; they never change the selection.
  import type { Send } from '../bridge';
  import type { View } from '../ipc';
  import { GROUPS, choiceOf, groupOf, matchesFilter, plural } from '../selection';
  import ModRow from './ModRow.svelte';

  let { view, send }: { view: View; send: Send } = $props();

  let filter = $state('');
  let chosenOnly = $state(false);

  const chosenCount = $derived(view.selection.want.length + view.selection.avoid.length);
  const shown = $derived(
    view.mods.filter((mod) => matchesFilter(mod, filter) && (!chosenOnly || choiceOf(view.selection, mod.id) !== 'none')),
  );
  const groups = $derived(
    GROUPS.map((g) => {
      const all = view.mods.filter((mod) => groupOf(mod) === g.id);
      return {
        ...g,
        total: all.length,
        wanted: all.filter((mod) => view.selection.want.includes(mod.id)).length,
        avoided: all.filter((mod) => view.selection.avoid.includes(mod.id)).length,
        rows: shown.filter((mod) => groupOf(mod) === g.id),
      };
    }),
  );
  const unmatchable = $derived(new Set(view.result.unmatchable));

  function clearChoices(): void {
    send('update_selection', { patch: { want: [], avoid: [] } });
  }
</script>

<section class="list" aria-label="Mods">
  <div class="toolbar">
    <label class="search">
      <span class="visually-hidden">Filter mods</span>
      <input class="field mod-filter" type="search" placeholder="Filter mods" bind:value={filter} autocomplete="off" spellcheck="false" />
    </label>
    <span class="count" aria-live="polite">{shown.length} of {plural(view.mods.length, 'mod', 'mods')}</span>
    <label class="chosen">
      <input class="chosen-only" type="checkbox" bind:checked={chosenOnly} />
      Chosen only
    </label>
    <button type="button" class="btn clear-choices" disabled={chosenCount === 0} onclick={clearChoices}>Clear choices</button>
  </div>

  <div class="scroll">
    {#each groups as g (g.id)}
      <section class="group" data-group={g.id} aria-labelledby="group-{g.id}">
        <h2 id="group-{g.id}">
          <span class="caps">{g.title}</span>
          <span class="group-count">
            {plural(g.total, 'mod', 'mods')}{#if g.wanted > 0}<span class="sep" aria-hidden="true">·</span><span class="wanted">{g.wanted} wanted</span>{/if}{#if g.avoided > 0}<span class="sep" aria-hidden="true">·</span><span class="avoided">{g.avoided} avoided</span>{/if}
          </span>
        </h2>
        {#if g.rows.length > 0}
          <ul>
            {#each g.rows as mod (mod.id)}
              <ModRow {mod} selection={view.selection} unmatchable={unmatchable.has(mod.id)} {send} />
            {/each}
          </ul>
        {:else}
          <p class="none">{g.total === 0 ? 'No mods in this group.' : chosenOnly && filter.trim() === '' ? 'None chosen here.' : 'No mod here matches the filter.'}</p>
        {/if}
      </section>
    {/each}
  </div>
</section>

<style>
  .list {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    min-height: 0;
  }

  .toolbar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: calc(var(--space-unit) * 2) calc(var(--space-unit) * 4);
    padding: calc(var(--space-unit) * 3) calc(var(--space-unit) * 5);
    border-bottom: var(--border-width) solid var(--color-panel-divider);
  }

  .search {
    flex: 1 1 220px;
    max-width: 360px;
  }

  .mod-filter {
    width: 100%;
  }

  .count {
    color: var(--color-text-muted);
    font-size: var(--size-caption);
    font-variant-numeric: tabular-nums;
  }

  .chosen {
    display: inline-flex;
    align-items: center;
    gap: calc(var(--space-unit) * 1.5);
    min-height: var(--size-button);
    color: var(--color-text-secondary);
    font-size: var(--size-label);
    cursor: pointer;
  }

  .chosen input {
    width: 16px;
    height: 16px;
    margin: 0;
    accent-color: var(--color-text-secondary);
  }

  .clear-choices {
    margin-left: auto;
  }

  .scroll {
    overflow-y: auto;
    padding: 0 calc(var(--space-unit) * 5) calc(var(--space-unit) * 4);
  }

  .group + .group {
    margin-top: calc(var(--space-unit) * 2);
  }

  h2 {
    position: sticky;
    top: 0;
    z-index: 1;
    display: flex;
    align-items: baseline;
    gap: calc(var(--space-unit) * 3);
    margin: 0;
    padding: calc(var(--space-unit) * 4) 0 calc(var(--space-unit) * 2);
    background: var(--color-panel-bg);
    border-bottom: var(--border-width) solid var(--color-panel-divider);
    font-size: var(--size-title);
    line-height: var(--line-title);
    font-weight: 400;
  }

  .group-count {
    color: var(--color-text-muted);
    font-family: var(--font-family);
    font-size: var(--size-caption);
    font-weight: 400;
  }

  .sep {
    margin: 0 calc(var(--space-unit) * 1.5);
  }

  .wanted {
    color: var(--want);
  }

  .avoided {
    color: var(--avoid);
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .none {
    margin: 0;
    padding: calc(var(--space-unit) * 3) 0;
    color: var(--color-text-muted);
    font-size: var(--size-step);
  }
</style>
