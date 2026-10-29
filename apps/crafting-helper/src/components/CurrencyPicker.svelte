<script lang="ts">
  // The currency picker (`ui-design.md` § "The currency picker"), shared by
  // the craft window's "Blocked for this craft" and the Settings window's
  // blocked currencies: the player picks by name from the catalogue, in its
  // groups, and never types an id. The filter text and "Blocked only" are
  // the picker's own state; which ids are checked is the caller's (from the
  // view), and a change hands the caller the whole new list, which it sends.
  // The box shows the view's state until the next view says otherwise.
  import type { CatalogueGroup, CatalogueEntry } from '../ipc';

  let {
    groups,
    selected,
    tags = {},
    onchange,
  }: {
    /** The catalogue (`get_catalogue`), in its groups. */
    groups: CatalogueGroup[];
    /** The ids checked now, in their order. */
    selected: readonly string[];
    /** A tag after an entry's name, by id (none in Settings). */
    tags?: Record<string, string>;
    /** The whole list after one change: the ids checked, a new one last. */
    onchange: (selected: string[]) => void;
  } = $props();

  let filter = $state('');
  let blockedOnly = $state(false);

  const checked = $derived(new Set(selected));
  const needle = $derived(filter.trim().toLocaleLowerCase('en'));
  const shows = (entry: CatalogueEntry): boolean =>
    (needle === '' || entry.name.toLocaleLowerCase('en').includes(needle)) && (!blockedOnly || checked.has(entry.id));
  const anyShown = $derived(groups.some((group) => group.entries.some(shows)));

  function toggle(event: Event, id: string): void {
    const box = event.currentTarget as HTMLInputElement;
    const next = box.checked ? [...selected.filter((other) => other !== id), id] : selected.filter((other) => other !== id);
    box.checked = checked.has(id);
    onchange(next);
  }
</script>

<div class="picker">
  <div class="tools">
    <input class="field filter" type="search" aria-label="Filter currencies" placeholder="Filter currencies" bind:value={filter} />
    <label class="blocked-only"><input type="checkbox" bind:checked={blockedOnly} /> Blocked only</label>
  </div>
  <div class="list">
    {#each groups as group (group.kind)}
      <fieldset class="picker-group" hidden={!group.entries.some(shows)}>
        <legend class="caps">{group.label}</legend>
        <ul>
          {#each group.entries as entry (entry.id)}
            <li class="entry" hidden={!shows(entry)}>
              <label>
                <input type="checkbox" data-id={entry.id} checked={checked.has(entry.id)} onchange={(event) => toggle(event, entry.id)} />
                <span class="name">{entry.name}</span>
                {#if tags[entry.id]}<span class="tag">{tags[entry.id]}</span>{/if}
              </label>
            </li>
          {/each}
        </ul>
      </fieldset>
    {/each}
    {#if !anyShown && needle !== ''}
      <p class="no-match">No currency matches “{filter.trim()}”.</p>
    {/if}
  </div>
</div>

<style>
  .picker {
    display: flex;
    flex-direction: column;
    gap: calc(var(--space-unit) * 2);
  }

  .tools {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: calc(var(--space-unit) * 2) calc(var(--space-unit) * 4);
  }

  .filter {
    flex: 1 1 180px;
    min-width: 0;
  }

  .blocked-only {
    display: inline-flex;
    align-items: center;
    gap: calc(var(--space-unit) * 1.5);
    color: var(--color-text-secondary);
    font-size: var(--size-step);
    cursor: pointer;
  }

  /* The list scrolls past 240 px, so a long catalogue never pushes the route away. */
  .list {
    max-height: 240px;
    overflow-y: auto;
    padding: var(--space-unit) calc(var(--space-unit) * 2);
    border: var(--border-width) solid var(--color-panel-border);
    background: var(--color-panel-sunken);
  }

  fieldset {
    margin: 0;
    padding: 0;
    border: 0;
    min-width: 0;
  }

  fieldset + fieldset {
    margin-top: calc(var(--space-unit) * 2);
  }

  legend {
    padding: 0;
    color: var(--color-text-muted);
    font-size: var(--size-caption);
    line-height: var(--line-caption);
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  label {
    display: flex;
    align-items: baseline;
    gap: calc(var(--space-unit) * 2);
    padding: 2px 0;
    font-size: var(--size-step);
    line-height: var(--line-step);
    cursor: pointer;
  }

  label input {
    align-self: center;
  }

  .name {
    color: var(--color-text-primary);
  }

  .no-match {
    margin: var(--space-unit) 0;
    color: var(--color-text-muted);
    font-size: var(--size-step);
  }
</style>
