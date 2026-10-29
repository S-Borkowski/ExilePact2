<script lang="ts">
  // The Blocked currencies section (`ui-design.md` § "Settings window"): the
  // help line, the count, and the currency picker over the saved list,
  // without tags. The player picks by name from the catalogue; there is no
  // field to type an id. Every change sends the whole list with
  // `update_settings`; the app keeps only catalogue ids, without repeats, and
  // sends the list it kept, which is what the picker checks.
  import CurrencyPicker from '../components/CurrencyPicker.svelte';
  import type { SectionProps } from './sections';

  let { view, send, catalogue }: SectionProps = $props();

  const count = $derived(view.blocked_currencies.length);

  function sendList(list: string[]): void {
    send('update_settings', { patch: { blocked_currencies: list } });
  }
</script>

<p class="help">Currencies the helper never suggests. Pick them by name.</p>
<p class="count caption">{count === 0 ? 'No currency is blocked.' : `${count} blocked`}</p>

<div class="picker-wrap">
  <CurrencyPicker groups={catalogue} selected={view.blocked_currencies} onchange={sendList} />
</div>

<style>
  .help {
    margin: 0 0 var(--space-unit);
    color: var(--color-text-muted);
    font-size: var(--size-label);
    line-height: var(--line-step);
    letter-spacing: 0.2px;
  }

  .count {
    margin: 0 0 var(--space-section-gap);
  }

  /* The window is a fixed 440 × 720: a shorter list (about six names) keeps
     Updates, its Check now and the footer on screen without scrolling. */
  .picker-wrap {
    --picker-list-height: 136px;
    margin-bottom: calc(var(--space-unit) * 3);
  }
</style>
