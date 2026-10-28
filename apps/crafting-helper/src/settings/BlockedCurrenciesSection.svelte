<script lang="ts">
  // The Blocked currencies section: a field with *Add*, and the saved list
  // with *Remove* on each id. Every change sends the whole list with
  // `update_settings`; the app trims, drops empty ids and duplicates, saves
  // it and sends the list it kept, which is what the section shows. The ids
  // are plain text for now; the crafting data names them later.
  import type { SectionProps } from './sections';

  let { view, send }: SectionProps = $props();

  /** The app's limits (`crates/ch-app/src/settings.rs`): it refuses a longer id or a longer list. */
  const MAX_ID_CHARS = 64;
  const MAX_IDS = 200;

  let draft = $state('');
  const wanted = $derived(draft.trim());
  const full = $derived(view.blocked_currencies.length >= MAX_IDS);
  const known = $derived(view.blocked_currencies.includes(wanted));

  function sendList(list: string[]): void {
    send('update_settings', { patch: { blocked_currencies: list } });
  }

  function add(event: SubmitEvent): void {
    event.preventDefault();
    if (wanted === '' || full) return;
    if (!known) sendList([...view.blocked_currencies, wanted]);
    draft = '';
  }

  function remove(id: string): void {
    sendList(view.blocked_currencies.filter((other) => other !== id));
  }
</script>

<p class="help" id="blocked-help">Currencies the helper never suggests. Each is a currency id.</p>

<form class="add" onsubmit={add}>
  <label class="visually-hidden" for="blocked-field">Currency id</label>
  <input
    id="blocked-field"
    class="field"
    type="text"
    placeholder="Currency id"
    autocomplete="off"
    spellcheck="false"
    maxlength={MAX_ID_CHARS}
    aria-describedby="blocked-help"
    disabled={full}
    bind:value={draft}
  />
  <button type="submit" class="btn" disabled={wanted === '' || full}>Add</button>
</form>

{#if view.blocked_currencies.length === 0}
  <p class="empty caption">No currency is blocked.</p>
{:else}
  <ul class="blocked" aria-label="Blocked currencies">
    {#each view.blocked_currencies as id (id)}
      <li class="item" data-id={id}>
        <span class="id">{id}</span>
        <button type="button" class="btn remove" aria-label={`Remove ${id}`} onclick={() => remove(id)}>Remove</button>
      </li>
    {/each}
  </ul>
  {#if full}
    <p class="caption">The list holds at most {MAX_IDS} ids.</p>
  {/if}
{/if}

<style>
  .help {
    margin: 0 0 var(--space-section-gap);
    color: var(--color-text-muted);
    font-size: var(--size-label);
    line-height: var(--line-step);
    letter-spacing: 0.2px;
  }

  .add {
    display: flex;
    gap: var(--space-section-gap);
  }

  .add .field {
    flex: 1;
    min-width: 0;
    font-family: var(--mono);
    user-select: text;
  }

  .empty {
    padding: calc(var(--space-unit) * 2) 0 calc(var(--space-unit) * 3);
  }

  .blocked {
    max-height: 200px;
    margin: var(--space-section-gap) 0 calc(var(--space-unit) * 3);
    padding: 0;
    overflow-y: auto;
    border: var(--border-width) solid var(--color-panel-divider);
    list-style: none;
  }

  .item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-row-pad);
    padding: var(--space-unit) var(--space-unit) var(--space-unit) calc(var(--space-unit) * 2);
  }

  .item + .item {
    border-top: var(--border-width) solid var(--color-panel-divider);
  }

  .id {
    min-width: 0;
    overflow-wrap: anywhere;
    font-family: var(--mono);
    font-size: var(--size-step);
    user-select: text;
  }

  .remove {
    flex: none;
  }
</style>
