<script lang="ts">
  // Targets (`ui-design.md` § "Mode planning"): the chosen targets with
  // their floors, then the choices for this base in three groups, narrowed
  // by the page's own filter. Every change (check, uncheck, a floor,
  // Remove, Clear targets) sends `set_targets` with the whole list; the
  // controls show the view's state until the next view says otherwise, so a
  // refused list never looks taken. The floor labels are Rust's, and so is
  // a choice's `disabled` reason (its side is full): that row stays listed,
  // greyed, its checkbox disabled, the reason its tooltip and description.
  import type { Send, TargetPick } from '../bridge';
  import type { ChosenTarget, FloorOption, PlanningView, TargetChoice } from '../ipc';

  let { planning, send }: { planning: PlanningView; send: Send } = $props();

  const uid = $props.id();
  let filter = $state('');

  const picks = $derived<TargetPick[]>(planning.targets.map((t) => ({ mod_id: t.mod_id, floor: t.floor })));
  const chosen = $derived(new Map(planning.targets.map((t) => [t.mod_id, t])));
  const needle = $derived(filter.trim().toLocaleLowerCase('en'));
  const shows = (choice: TargetChoice): boolean => needle === '' || choice.text.toLocaleLowerCase('en').includes(needle);
  const groups = $derived(
    (
      [
        { key: 'prefixes', label: 'Prefixes', choices: planning.choices.prefixes },
        { key: 'suffixes', label: 'Suffixes', choices: planning.choices.suffixes },
        { key: 'desecrated', label: 'Desecrated', choices: planning.choices.desecrated },
      ] as const
    ).filter((group) => group.choices.length > 0),
  );
  const anyShown = $derived(groups.some((group) => group.choices.some(shows)));
  /** The floor options of each target, from its choice (the view's). */
  const floorsOf = $derived(
    new Map(
      [...planning.choices.prefixes, ...planning.choices.suffixes, ...planning.choices.desecrated].map((c) => [c.mod_id, c.floors]),
    ),
  );

  const optionValue = (floor: number | null): string => (floor === null ? '' : String(floor));

  function setTargets(targets: TargetPick[]): void {
    send('set_targets', { targets });
  }

  function toggle(event: Event, choice: TargetChoice): void {
    const box = event.currentTarget as HTMLInputElement;
    const next = box.checked
      ? [...picks.filter((p) => p.mod_id !== choice.mod_id), { mod_id: choice.mod_id, floor: null }]
      : picks.filter((p) => p.mod_id !== choice.mod_id);
    box.checked = chosen.has(choice.mod_id);
    setTargets(next);
  }

  function setFloor(event: Event, target: ChosenTarget): void {
    const select = event.currentTarget as HTMLSelectElement;
    const floor = select.value === '' ? null : Number(select.value);
    select.value = optionValue(target.floor);
    setTargets(picks.map((p) => (p.mod_id === target.mod_id ? { mod_id: p.mod_id, floor } : p)));
  }
</script>

{#snippet floorSelect(target: ChosenTarget, floors: FloorOption[])}
  <select
    class="field floor"
    aria-label="Floor for {target.text}"
    value={optionValue(target.floor)}
    onchange={(event) => setFloor(event, target)}
  >
    {#each floors as option (option.label)}
      <option value={optionValue(option.floor)}>{option.label}</option>
    {/each}
  </select>
{/snippet}

<section class="targets" aria-labelledby="{uid}-title">
  <h2 class="section-title caps" id="{uid}-title">Targets</h2>
  <p class="help">T1 is the best tier; a floor keeps a roll at that tier or better.</p>

  <div class="chosen">
    <div class="chosen-head">
      <h3 class="section-title caps" id="{uid}-chosen">Chosen</h3>
      <button type="button" class="btn" disabled={planning.targets.length === 0} onclick={() => setTargets([])}>Clear targets</button>
    </div>
    {#if planning.targets.length === 0}
      <p class="chosen-empty">Pick the modifiers you want from the list below.</p>
    {:else}
      <ul aria-labelledby="{uid}-chosen">
        {#each planning.targets as target (target.mod_id)}
          <li>
            <span class="target-text">{target.text}</span>
            {@render floorSelect(target, floorsOf.get(target.mod_id) ?? [])}
            <button
              type="button"
              class="btn remove"
              aria-label="Remove {target.text}"
              onclick={() => setTargets(picks.filter((p) => p.mod_id !== target.mod_id))}>Remove</button
            >
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  <input class="field filter" type="search" aria-label="Filter modifiers" placeholder="Filter modifiers" bind:value={filter} />

  <div class="choice-list">
    {#each groups as group (group.key)}
      {@const count = group.choices.filter((c) => chosen.has(c.mod_id)).length}
      <fieldset class="choices" data-group={group.key} hidden={!group.choices.some(shows)}>
        <legend class="section-title caps">{group.label} · {count} chosen</legend>
        <ul>
          {#each group.choices as choice (choice.mod_id)}
            {@const target = chosen.get(choice.mod_id)}
            {@const why = target === undefined ? choice.disabled : null}
            <li hidden={!shows(choice)} class:disabled={why !== null} title={why ?? undefined}>
              <label>
                <input
                  type="checkbox"
                  checked={target !== undefined}
                  disabled={why !== null}
                  aria-describedby={why !== null ? `${uid}-why-${choice.mod_id}` : undefined}
                  onchange={(event) => toggle(event, choice)}
                />
                <span class="name">{choice.text}</span>
                <span class="best">best T{choice.best_tier}</span>
              </label>
              {#if why !== null}
                <span class="visually-hidden" id="{uid}-why-{choice.mod_id}">{why}</span>
              {/if}
              {#if target}
                {@render floorSelect(target, choice.floors)}
              {/if}
            </li>
          {/each}
        </ul>
      </fieldset>
    {/each}
    {#if !anyShown && needle !== ''}
      <p class="no-match">No modifier matches “{filter.trim()}”.</p>
    {/if}
  </div>
</section>

<style>
  .targets {
    display: flex;
    flex-direction: column;
    gap: calc(var(--space-unit) * 2);
  }

  .help,
  .chosen-empty,
  .no-match {
    margin: 0;
    color: var(--color-text-muted);
    font-size: var(--size-step);
    line-height: var(--line-step);
  }

  .chosen {
    display: flex;
    flex-direction: column;
    gap: var(--space-unit);
    padding: calc(var(--space-unit) * 2) calc(var(--space-unit) * 3);
    border: var(--border-width) solid var(--color-panel-border);
    background: var(--color-panel-raised);
  }

  .chosen-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: calc(var(--space-unit) * 2);
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  /* One row per target: its text (wrapping when long), the floor, Remove.
     The text gives way, so Remove never drops to a line of its own. */
  .chosen li {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto auto;
    align-items: center;
    gap: calc(var(--space-unit) * 2);
    padding: var(--space-unit) 0;
  }

  .chosen li + li {
    border-top: var(--border-width) solid var(--color-panel-divider);
  }

  .target-text {
    min-width: 0;
    color: var(--color-text-primary);
    font-size: var(--size-step);
    line-height: var(--line-step);
  }

  .floor {
    max-width: 100%;
  }

  .filter {
    width: 100%;
  }

  .choice-list {
    display: flex;
    flex-direction: column;
    gap: calc(var(--space-unit) * 2);
  }

  /* In one column (below the craft area's 880 px split) the choices scroll
     in their own sunken box, as the currency list does, so the route under
     them is not a whole base's modifier list away. */
  @media (max-width: 879px) {
    .choice-list {
      max-height: 240px;
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

  .choices li {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-unit) calc(var(--space-unit) * 2);
    padding: 2px 0;
  }

  label {
    display: flex;
    flex: 1 1 200px;
    align-items: baseline;
    gap: calc(var(--space-unit) * 2);
    min-width: 0;
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

  /* A choice its side has no room for: greyed, still listed and filtered,
     its reason the row's tooltip and the checkbox's description. */
  .choices li.disabled label {
    cursor: not-allowed;
  }

  .choices li.disabled .name {
    color: var(--color-text-muted);
  }

  .best {
    color: var(--color-text-muted);
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }
</style>
