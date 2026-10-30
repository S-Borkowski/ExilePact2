<script lang="ts">
  // The Trade section (`ui-design.md` § "Settings window"): the league the
  // trade panel's search opens in, a select of the leagues the view offers
  // (Rust's list: the shipped one until a refresh this run, the saved league
  // always among them), and beside it the refresh icon, which sends
  // `refresh_trade_leagues` with nothing. The app makes that one request,
  // only on this click; the page itself reaches nothing. A change of league
  // sends `update_settings { trade_league }`; the select shows the view's
  // league until the next view says otherwise, so a refused name never
  // looks saved. The note line is Rust's, or the page's own words while a
  // refresh runs.
  import type { SectionProps } from './sections';

  let { view, send }: SectionProps = $props();

  const refreshing = $derived(view.trade_leagues_refreshing);
  const REFRESH_NAME = 'Refresh the league list from the trade site';

  function chooseLeague(event: Event): void {
    const select = event.currentTarget as HTMLSelectElement;
    const league = select.value;
    select.value = view.trade_league;
    if (league !== view.trade_league) send('update_settings', { patch: { trade_league: league } });
  }

  function refresh(): void {
    if (refreshing) return;
    send('refresh_trade_leagues', {});
  }
</script>

<div class="row">
  <label class="label" for="trade-league">League</label>
  <div class="pick">
    <select
      id="trade-league"
      class="field"
      aria-describedby="trade-league-help trade-league-note"
      value={view.trade_league}
      onchange={chooseLeague}
    >
      {#each view.trade_leagues as league (league)}
        <option value={league} selected={league === view.trade_league}>{league}</option>
      {/each}
    </select>
    <button
      type="button"
      class="btn icon"
      aria-label={REFRESH_NAME}
      title={REFRESH_NAME}
      disabled={refreshing}
      aria-busy={refreshing ? 'true' : undefined}
      onclick={refresh}
    >
      <!-- A circular arrow; it stays still while a refresh runs. -->
      <svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" focusable="false">
        <path d="M13 8a5 5 0 1 1-1.46-3.54" fill="none" stroke="currentColor" stroke-width="1.5" />
        <path d="M12.5 1.75v3.25H9.25" fill="none" stroke="currentColor" stroke-width="1.5" />
      </svg>
    </button>
  </div>
</div>
<p class="help" id="trade-league-help">
  The league the trade search opens in. The refresh button asks the trade site (pathofexile.com) for its current leagues; nothing is sent
  until you click it, and the new list lasts until the app closes.
</p>
<p
  class="status"
  id="trade-league-note"
  role="status"
  aria-live="polite"
  data-kind={refreshing ? 'info' : (view.trade_league_note?.kind ?? 'none')}
>
  {#if refreshing}
    <span>Refreshing the league list…</span>
  {:else if view.trade_league_note}
    {#if view.trade_league_note.kind === 'error'}<span class="mark" aria-hidden="true">!</span>{/if}<span
      >{view.trade_league_note.text}</span
    >
  {/if}
</p>

<style>
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-row-pad);
    min-height: var(--size-row-height);
  }

  .label {
    color: var(--color-text-primary);
    font-size: var(--size-label);
    font-weight: 500;
    line-height: var(--line-caption);
  }

  /* The select stretches to the refresh icon's 32 px, so the two controls on
     the row share one height, a step apart. */
  .pick {
    display: flex;
    align-items: stretch;
    gap: var(--space-section-gap);
    min-width: 0;
  }

  .pick .field {
    min-width: 152px;
    max-width: 240px;
  }

  /* The refresh icon: a square secondary button, 32 × 32, its 16 px icon
     at least one space unit clear of every edge. */
  .icon {
    flex: none;
    width: 32px;
    height: 32px;
    min-height: 32px;
    padding: var(--space-unit);
  }

  .icon[aria-busy='true'] {
    cursor: progress;
  }

  .help {
    margin: calc(var(--space-unit) * -1.5) 0 0;
    color: var(--color-text-muted);
    font-size: var(--size-label);
    line-height: var(--line-step);
    letter-spacing: 0.2px;
  }

  /* A row, so a wrapped refusal hangs its lines after the "!". */
  .status {
    display: flex;
    align-items: baseline;
    min-height: var(--line-step);
    margin: 0;
    padding: var(--space-unit) 0 calc(var(--space-unit) * 3);
    color: var(--color-text-secondary);
    font-size: var(--size-step);
    line-height: var(--line-step);
  }

  .status[data-kind='error'] {
    color: var(--error);
  }

  .mark {
    flex: none;
    margin-right: calc(var(--space-unit) * 1.5);
    font-weight: 700;
  }
</style>
