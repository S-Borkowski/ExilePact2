<script lang="ts">
  // The *Find a base on trade* panel (`ui-design.md` § "Find a base on
  // trade"), in the craft area's place while the view's trade part is open.
  // Every change sends `set_trade_search` with the whole selection, as the
  // view holds it with one field changed; Rust validates it and sends the
  // next view, and a refusal leaves the selection as it was. The page holds
  // no address and builds no link: the link is Rust's, shown read-only as
  // text, and the Search button sends `open_trade_search` with nothing, so
  // the app opens what it built from its own state.
  //
  // Layout: from 880 px two columns, about 45 % and 55 %, each scrolling on
  // its own (Base, Match, League and the link left; Modifiers right), the
  // Search bar at the left column's foot; below, one column scrolling as a
  // whole (Base, Modifiers, Match, League and the link), the bar at the
  // window's foot.
  import type { Send } from '../bridge';
  import type { TradePickChoice, TradeSelection, TradeView } from '../ipc';
  import TradeModifiers from './TradeModifiers.svelte';

  let { trade, send }: { trade: TradeView; send: Send } = $props();

  const uid = $props.id();

  const sel = $derived(trade.selection);

  // The item level the player has typed but the view does not hold yet:
  // `typed`, the field's text since it was last committed; `sent`, the level
  // committed while `view` was the view on screen, until the next view
  // answers. Without them a pick made before Rust's answer was sent with the
  // view's old level and undid the one just typed (Live 2).
  let typed: string | null = null;
  let sent: { view: TradeView; level: number | null } | null = null;

  /** A field's text as a level to send: empty is none, `undefined` is not a whole number. */
  function parseLevel(raw: string): number | null | undefined {
    const text = raw.trim();
    if (text === '') return null;
    const level = Number(text);
    return Number.isInteger(level) && level >= 0 && level <= 0xffff_ffff ? level : undefined;
  }

  /** The level to send with any change: what was typed, else what was sent, else the view's. */
  function currentLevel(): number | null {
    const level = typed === null ? undefined : parseLevel(typed);
    if (level !== undefined) return level;
    if (sent !== null && sent.view === trade) return sent.level;
    return sel.item_level;
  }

  /** The view's selection as plain objects, with `change` applied: what `set_trade_search` sends. */
  function selectionWith(change: Partial<TradeSelection>): TradeSelection {
    return {
      open: sel.open,
      class: sel.class,
      base: sel.base,
      item_level: currentLevel(),
      picks: sel.picks.map((p) => ({ mod_id: p.mod_id, floor: p.floor })),
      both_slots: sel.both_slots,
      ...change,
    };
  }

  function setSearch(change: Partial<TradeSelection>): void {
    send('set_trade_search', { search: selectionWith(change) });
  }

  function chooseClass(event: Event): void {
    const select = event.currentTarget as HTMLSelectElement;
    const value = select.value === '' ? null : select.value;
    select.value = sel.class ?? '';
    // Rust clears the base and the picks for a new class, and says how many.
    setSearch({ class: value });
  }

  function chooseBase(event: Event): void {
    const select = event.currentTarget as HTMLSelectElement;
    const value = select.value === '' ? null : select.value;
    select.value = sel.base ?? '';
    setSearch({ base: value });
  }

  /**
   * The minimum item level, sent on change (Enter, or leaving the field):
   * empty is none. A whole number goes to Rust, which takes 1 to 100 and
   * says why it refuses anything else; what is not a whole number at all
   * cannot be sent as one, so the field shows the view's value again.
   */
  function setItemLevel(event: Event): void {
    const field = event.currentTarget as HTMLInputElement;
    const level = parseLevel(field.value);
    typed = null;
    field.value = sel.item_level === null ? '' : String(sel.item_level);
    if (level === undefined || (level === null && sel.item_level === null)) return;
    sent = { view: trade, level };
    setSearch({ item_level: level });
  }

  /** What is typed is kept, and goes with any change made before it is committed. */
  function typeItemLevel(event: Event): void {
    typed = (event.currentTarget as HTMLInputElement).value;
  }

  function setBothSlots(event: Event, both: boolean): void {
    // The group shows the view's choice until the next view says otherwise.
    const group = (event.currentTarget as HTMLInputElement).closest('fieldset');
    for (const radio of group?.querySelectorAll<HTMLInputElement>('input[type="radio"]') ?? []) {
      radio.checked = (radio.value === 'both') === sel.both_slots;
    }
    setSearch({ both_slots: both });
  }

  const pick = (picks: TradePickChoice[]): void => setSearch({ picks });
</script>

<div class="trade" data-trade>
  <div class="head">
    <button type="button" class="btn back" onclick={() => setSearch({ open: false })}>
      <svg viewBox="0 0 12 12" width="12" height="12" aria-hidden="true" focusable="false">
        <path d="M7.5 2.5 4 6l3.5 3.5" fill="none" stroke="currentColor" stroke-width="1.5" />
      </svg>
      <span>Back to the craft</span>
    </button>
    <h2 class="title caps" tabindex="-1" data-mode-heading data-trade-heading>Find a base on trade</h2>
  </div>

  <div class="scroll">
    <div class="col left">
      <section class="group base" aria-labelledby="{uid}-base">
        <h3 class="section-title caps" id="{uid}-base">Base</h3>
        <div class="field-row">
          <label class="label" for="{uid}-class">Item class</label>
          <select id="{uid}-class" class="field" value={sel.class ?? ''} onchange={chooseClass}>
            <option value="" selected={sel.class === null}>Choose a class</option>
            {#each trade.classes as name (name)}
              <option value={name} selected={name === sel.class}>{name}</option>
            {/each}
          </select>
        </div>
        <div class="field-row">
          <label class="label" for="{uid}-base-type">Base type</label>
          <select id="{uid}-base-type" class="field" value={sel.base ?? ''} disabled={sel.class === null} onchange={chooseBase}>
            <option value="" selected={sel.base === null}>Choose a base</option>
            {#each trade.bases as name (name)}
              <option value={name} selected={name === sel.base}>{name}</option>
            {/each}
          </select>
        </div>
        <div class="field-row">
          <label class="label" for="{uid}-level">Minimum item level</label>
          <input
            id="{uid}-level"
            class="field level"
            type="number"
            inputmode="numeric"
            min="1"
            max="100"
            step="1"
            autocomplete="off"
            aria-describedby="{uid}-level-help"
            value={sel.item_level === null ? '' : String(sel.item_level)}
            oninput={typeItemLevel}
            onchange={setItemLevel}
          />
        </div>
        <p class="help" id="{uid}-level-help">Optional. Empty searches any item level; a higher level reaches better tiers.</p>
      </section>

      <fieldset class="group match" aria-describedby={trade.count_line ? `${uid}-count` : undefined}>
        <legend class="section-title caps">Match</legend>
        <label class="choice">
          <input type="radio" name="{uid}-match" value="both" checked={sel.both_slots} onchange={(e) => setBothSlots(e, true)} />
          <span class="choice-text">
            <span class="choice-name">Both slots</span>
            <span class="hint">A prefix and a suffix from your picks.</span>
          </span>
        </label>
        <label class="choice">
          <input type="radio" name="{uid}-match" value="one" checked={!sel.both_slots} onchange={(e) => setBothSlots(e, false)} />
          <span class="choice-text">
            <span class="choice-name">At least one</span>
            <span class="hint">Any one of your picks.</span>
          </span>
        </label>
        {#if trade.count_line}
          <p class="count-line" id="{uid}-count">{trade.count_line}</p>
        {/if}
      </fieldset>

      <section class="group league-link" aria-label="League and trade link">
        <p class="league">
          <span class="league-label caps">League</span>
          <span class="league-name">{trade.league}</span>
          <button type="button" class="btn small" onclick={() => send('open_settings', {})}>Change</button>
        </p>
        {#if trade.link !== null}
          <label class="label" for="{uid}-link">Trade link</label>
          <textarea
            id="{uid}-link"
            class="field link"
            readonly
            rows="3"
            spellcheck="false"
            aria-describedby="{uid}-link-help"
            value={trade.link}
          ></textarea>
          <p class="help" id="{uid}-link-help">Select it and press Ctrl+C to copy it.</p>
        {/if}
      </section>
    </div>

    <div class="col right">
      <TradeModifiers {trade} {pick} />
    </div>
  </div>

  <div class="bar search-bar">
    <!-- Sends the command alone: the app opens the link it built from its
         own state in the default browser, and this page holds none. -->
    <button
      type="button"
      class="btn primary search"
      aria-describedby="{uid}-opens"
      disabled={!trade.can_search}
      onclick={() => send('open_trade_search', {})}
    >
      <span>Search on the trade site</span>
      <svg class="out" viewBox="0 0 12 12" width="12" height="12" aria-hidden="true" focusable="false">
        <path d="M4.5 2.5h5v5M9.5 2.5 3 9" fill="none" stroke="currentColor" stroke-width="1.5" />
      </svg>
    </button>
    <span class="visually-hidden" id="{uid}-opens">Opens the trade site in your web browser</span>
    {#if trade.why_not}
      <p class="why" data-kind={trade.why_not.kind}>
        {#if trade.why_not.kind === 'error'}<span class="mark" aria-hidden="true">!</span>{/if}<span>{trade.why_not.text}</span>
      </p>
    {/if}
  </div>
</div>

<style>
  /* One column below 880 px: the head, the groups scrolling as a whole, the
     Search bar at the window's foot above the status line. */
  .trade {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr) auto;
    grid-template-areas:
      'head'
      'scroll'
      'bar';
    min-height: 0;
  }

  .head {
    grid-area: head;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: calc(var(--space-unit) * 2) calc(var(--space-unit) * 4);
    padding: calc(var(--space-unit) * 3) calc(var(--space-unit) * 5);
    border-bottom: var(--border-width) solid var(--color-panel-divider);
  }

  .title {
    margin: 0;
    color: var(--color-text-secondary);
    font-size: var(--size-act);
    line-height: var(--line-caption);
    letter-spacing: 0.03em;
  }

  .scroll {
    grid-area: scroll;
    display: flex;
    flex-direction: column;
    gap: calc(var(--space-section-gap) * 2);
    min-height: 0;
    overflow-y: auto;
    padding: calc(var(--space-unit) * 4) calc(var(--space-unit) * 5);
  }

  /* In one column the two columns' groups are one list, in the order Base,
     Modifiers, Match, League and the link. */
  .col {
    display: contents;
  }

  .base {
    order: 1;
  }

  .right :global(.modifiers) {
    order: 2;
  }

  .match {
    order: 3;
  }

  .league-link {
    order: 4;
  }

  .group {
    display: flex;
    flex-direction: column;
    gap: calc(var(--space-unit) * 2);
    min-width: 0;
  }

  fieldset {
    margin: 0;
    padding: 0;
    border: 0;
  }

  /* A rendered legend is no flex item, so the group's gap does not reach it:
     its padding gives it the same room a group's heading has. */
  legend {
    padding: 0 0 calc(var(--space-unit) * 2);
  }

  .field-row {
    display: grid;
    grid-template-columns: 9.5em minmax(0, 1fr);
    align-items: center;
    gap: calc(var(--space-unit) * 3);
  }

  .label {
    color: var(--color-text-primary);
    font-size: var(--size-label);
    font-weight: 500;
    line-height: var(--line-caption);
  }

  .field-row .field {
    width: 100%;
    min-width: 0;
  }

  .level {
    max-width: 8em;
    font-variant-numeric: tabular-nums;
  }

  .help,
  .hint,
  .count-line {
    margin: 0;
    color: var(--color-text-muted);
    font-size: var(--size-step);
    line-height: var(--line-step);
  }

  .count-line {
    color: var(--color-text-secondary);
  }

  .choice {
    display: flex;
    align-items: baseline;
    gap: calc(var(--space-unit) * 2);
    cursor: pointer;
  }

  /* The radio in the middle of its name's line (the native box is 13 px),
     with no native side margins, so it starts at the column's edge. */
  .choice input {
    flex: none;
    align-self: flex-start;
    margin: calc((var(--line-step) - 13px) / 2) 0 0;
  }

  .choice-text {
    display: flex;
    flex-direction: column;
  }

  .choice-name {
    color: var(--color-text-primary);
    font-size: var(--size-step);
    line-height: var(--line-step);
  }

  .hint {
    font-size: var(--size-label);
  }

  .league {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: calc(var(--space-unit) * 2) calc(var(--space-unit) * 3);
    margin: 0;
  }

  .league-label {
    color: var(--color-text-muted);
    font-size: var(--size-act);
    line-height: var(--line-caption);
    letter-spacing: 0.03em;
  }

  .league-name {
    min-width: 0;
    overflow-wrap: anywhere;
    color: var(--color-text-primary);
    font-size: var(--size-step);
    line-height: var(--line-step);
  }

  .small {
    min-height: var(--size-control);
    padding: 0 calc(var(--space-unit) * 2);
  }

  /* The link as text: a sunken read-only field in the system monospace,
     breaking anywhere, selectable so it can be copied by hand. */
  .link {
    width: 100%;
    min-height: auto;
    padding: var(--space-unit) calc(var(--space-unit) * 2);
    font-family: var(--mono);
    font-size: var(--size-caption);
    line-height: var(--line-caption);
    word-break: break-all;
    overflow-wrap: anywhere;
    resize: none;
    user-select: text;
  }

  .bar {
    grid-area: bar;
  }

  .search-bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: calc(var(--space-unit) * 3);
    padding: calc(var(--space-unit) * 3) calc(var(--space-unit) * 5);
    border-top: var(--border-width) solid var(--color-panel-divider);
    background: var(--color-panel-bg);
  }

  .out {
    flex: none;
  }

  .why {
    display: flex;
    align-items: baseline;
    margin: 0;
    color: var(--color-text-secondary);
    font-size: var(--size-step);
    line-height: var(--line-step);
  }

  .why[data-kind='error'] {
    color: var(--error);
  }

  .mark {
    flex: none;
    margin-right: calc(var(--space-unit) * 1.5);
    font-weight: 700;
  }

  /* From 880 px: two columns, about 45 % and 55 %, each scrolling on its
     own; the Search bar at the left column's foot. */
  @media (min-width: 880px) {
    .trade {
      grid-template-columns: 45fr 55fr;
      grid-template-rows: auto minmax(0, 1fr) auto;
      grid-template-areas:
        'head head'
        'left right'
        'bar right';
    }

    .scroll {
      display: contents;
    }

    .col {
      display: flex;
      flex-direction: column;
      gap: calc(var(--space-section-gap) * 2);
      min-width: 0;
      min-height: 0;
      overflow-y: auto;
      padding: calc(var(--space-unit) * 4) calc(var(--space-unit) * 5);
    }

    .left {
      grid-area: left;
    }

    .right {
      grid-area: right;
      border-left: var(--border-width) solid var(--color-panel-divider);
    }
  }
</style>
