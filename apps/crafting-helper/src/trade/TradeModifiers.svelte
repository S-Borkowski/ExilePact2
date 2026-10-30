<script lang="ts">
  // The trade panel's Modifiers (`ui-design.md` § "Find a base on trade",
  // "The controls"): the base's searchable prefixes and suffixes, each row a
  // checkbox with Rust's weight at its right end, a picked row's floor and,
  // for a default weight, Rust's reason; then the modifiers the trade site
  // cannot search, in one closed disclosure with no checkbox. The filter is
  // the page's own and sends nothing. A check, an uncheck or a floor sends
  // `set_trade_search` with the whole selection (through `pick`); each
  // control shows the view's state until the next view says otherwise, so a
  // refused pick never looks taken.
  import type { TradePickChoice, TradeModView, TradeView } from '../ipc';

  let { trade, pick }: { trade: TradeView; pick: (picks: TradePickChoice[]) => void } = $props();

  const uid = $props.id();
  let filter = $state('');

  const needle = $derived(filter.trim().toLocaleLowerCase('en'));
  const shows = (modText: string): boolean => needle === '' || modText.toLocaleLowerCase('en').includes(needle);
  const groups = $derived(
    (
      [
        { key: 'prefixes', label: 'Prefixes', mods: trade.prefixes },
        { key: 'suffixes', label: 'Suffixes', mods: trade.suffixes },
      ] as const
    ).filter((group) => group.mods.length > 0),
  );
  const anyShown = $derived(
    groups.some((group) => group.mods.some((m) => shows(m.text))) || trade.not_searchable.some((m) => shows(m.text)),
  );
  const anyDefault = $derived([...trade.prefixes, ...trade.suffixes].some((m) => m.weight_source === 'default'));
  /** The picks as the view holds them, plain objects, in the order picked. */
  const picks = $derived<TradePickChoice[]>(trade.selection.picks.map((p) => ({ mod_id: p.mod_id, floor: p.floor })));

  const optionValue = (floor: number | null): string => (floor === null ? '' : String(floor));

  function toggle(event: Event, mod: TradeModView): void {
    const box = event.currentTarget as HTMLInputElement;
    const next = box.checked
      ? [...picks.filter((p) => p.mod_id !== mod.mod_id), { mod_id: mod.mod_id, floor: null }]
      : picks.filter((p) => p.mod_id !== mod.mod_id);
    box.checked = mod.picked;
    pick(next);
  }

  function setFloor(event: Event, mod: TradeModView): void {
    const select = event.currentTarget as HTMLSelectElement;
    const floor = select.value === '' ? null : Number(select.value);
    select.value = optionValue(mod.floor);
    pick(picks.map((p) => (p.mod_id === mod.mod_id ? { mod_id: p.mod_id, floor } : p)));
  }
</script>

<section class="modifiers" aria-labelledby="{uid}-title">
  <h3 class="section-title caps" id="{uid}-title">Modifiers</h3>
  {#if trade.selection.class === null}
    <p class="prompt">Choose an item class to see its modifiers.</p>
  {:else if trade.selection.base === null}
    <p class="prompt">Choose a base type to see its modifiers.</p>
  {:else}
    <p class="help">
      Weight: higher is rarer. 12.0 means about 1 in 12 modifiers added to that side of this base is this one, at the floor or better.
      The search ranks results by these weights.
    </p>
    <input class="field filter" type="search" aria-label="Filter modifiers" placeholder="Filter modifiers" bind:value={filter} />
    {#if anyDefault}
      <p class="help legend">“default” marks a weight the data does not give; a picked modifier says why.</p>
    {/if}
    {#if trade.pick_limit}
      <p class="limit">{trade.pick_limit}</p>
    {/if}

    <div class="mod-list">
      {#each groups as group (group.key)}
        {@const count = group.mods.filter((m) => m.picked).length}
        <fieldset class="mod-group" data-group={group.key} hidden={!group.mods.some((m) => shows(m.text))}>
          <legend class="section-title caps">{group.label} · {count} picked</legend>
          <ul>
            {#each group.mods as mod (mod.mod_id)}
              <li hidden={!shows(mod.text)} data-mod={mod.mod_id} data-source={mod.weight_source}>
                <label>
                  <input
                    type="checkbox"
                    checked={mod.picked}
                    disabled={!mod.picked && !trade.can_pick}
                    onchange={(event) => toggle(event, mod)}
                  />
                  <!-- No white space between the parts: a screen reader hears
                       "<text>, weight 12.4" (", default" after a default). -->
                  <span class="name">{mod.text}</span><span class="weight-cell"
                    ><span class="visually-hidden">{', weight '}</span><span class="weight">{mod.weight}</span
                    >{#if mod.weight_source === 'default'}<span class="visually-hidden">{', '}</span><span class="tag">default</span
                      >{/if}</span
                  >
                </label>
                {#if mod.picked}
                  <div class="picked">
                    <select
                      class="field floor"
                      aria-label="Floor for {mod.text}"
                      value={optionValue(mod.floor)}
                      onchange={(event) => setFloor(event, mod)}
                    >
                      {#each mod.floors as option (option.label)}
                        <option value={optionValue(option.floor)} selected={option.floor === mod.floor}>{option.label}</option>
                      {/each}
                    </select>
                    {#if mod.weight_source === 'default' && mod.default_reason}
                      <p class="reason">default: {mod.default_reason}</p>
                    {/if}
                  </div>
                {/if}
              </li>
            {/each}
          </ul>
        </fieldset>
      {/each}

      {#if trade.not_searchable.length > 0}
        <details class="not-searchable" hidden={!trade.not_searchable.some((m) => shows(m.text))}>
          <summary>Not searchable on trade: {trade.not_searchable.length}</summary>
          <ul>
            {#each trade.not_searchable as mod (mod.mod_id)}
              <li hidden={!shows(mod.text)} data-mod={mod.mod_id}>
                <span class="name">{mod.text}</span>
                <span class="side">{mod.side}</span>
                <span class="reason">{mod.reason}</span>
              </li>
            {/each}
          </ul>
        </details>
      {/if}

      {#if !anyShown && needle !== ''}
        <p class="no-match">No modifier matches “{filter.trim()}”.</p>
      {/if}
    </div>
  {/if}
</section>

<style>
  .modifiers {
    display: flex;
    flex-direction: column;
    gap: calc(var(--space-unit) * 2);
    min-width: 0;
  }

  .prompt,
  .help,
  .no-match,
  .reason,
  .limit {
    margin: 0;
    color: var(--color-text-muted);
    font-size: var(--size-step);
    line-height: var(--line-step);
  }

  .limit {
    color: var(--color-text-secondary);
  }

  .filter {
    width: 100%;
  }

  .mod-list {
    display: flex;
    flex-direction: column;
    gap: calc(var(--space-unit) * 3);
  }

  /* In one column (below 880 px) the groups scroll in their own sunken box
     past 200 px, so Match and the link are not a whole base's list away. */
  @media (max-width: 879px) {
    .mod-list {
      max-height: 200px;
      overflow-y: auto;
      padding: var(--space-unit) calc(var(--space-unit) * 2);
      border: var(--border-width) solid var(--color-panel-border);
      background: var(--color-panel-sunken);
    }
  }

  fieldset {
    margin: 0;
    padding: 0;
    border: 0;
    min-width: 0;
  }

  legend {
    padding: 0 0 var(--space-unit);
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .mod-group li {
    padding: 2px 0;
  }

  .mod-group li + li {
    border-top: var(--border-width) solid var(--color-panel-divider);
  }

  /* One line: the checkbox, the text taking the room, the weight column at
     the right end in tabular figures so the numbers line up. */
  label {
    display: flex;
    align-items: baseline;
    gap: calc(var(--space-unit) * 2);
    min-width: 0;
    min-height: var(--size-button);
    font-size: var(--size-step);
    line-height: var(--line-step);
    cursor: pointer;
  }

  label input {
    flex: none;
    align-self: center;
  }

  label input:disabled {
    cursor: not-allowed;
  }

  label:has(input:disabled) {
    cursor: not-allowed;
  }

  .name {
    flex: 1;
    min-width: 0;
    color: var(--color-text-primary);
  }

  /* A searchable row's text sits in the middle of the room between the
     checkbox and the weight column, each wrapped line too. The
     not-searchable list's names stay at the left. */
  label .name {
    text-align: center;
  }

  .weight-cell {
    display: inline-flex;
    flex: none;
    align-items: baseline;
    gap: var(--space-unit);
  }

  .weight {
    min-width: 4ch;
    color: var(--color-text-secondary);
    font-variant-numeric: tabular-nums;
    text-align: right;
  }

  /* A picked row's floor and, for a default weight, its reason, under the
     text, indented to it. */
  .picked {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-unit);
    padding: 0 0 var(--space-unit) calc(13px + var(--space-unit) * 2);
  }

  .floor {
    max-width: 100%;
  }

  .not-searchable summary {
    color: var(--color-text-secondary);
    font-size: var(--size-step);
    line-height: var(--line-step);
    cursor: pointer;
  }

  .not-searchable ul {
    display: flex;
    flex-direction: column;
    gap: var(--space-unit);
    padding-top: var(--space-unit);
  }

  .not-searchable li {
    display: flex;
    flex-wrap: wrap;
    gap: 0 calc(var(--space-unit) * 2);
    font-size: var(--size-step);
    line-height: var(--line-step);
  }

  .side {
    color: var(--color-text-muted);
  }

  .not-searchable .reason {
    flex-basis: 100%;
  }
</style>
