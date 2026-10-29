<script lang="ts">
  // "Blocked for this craft" (`ui-design.md` § "Mode planning"): a
  // disclosure whose summary always counts the effective list, over the
  // currency picker. Closed by default, open when the previewed route has a
  // fallback or there is no route (where the blocked list is the likely
  // fix); the player's own opening or closing then holds while planning
  // lasts (this component lives only in that mode). A change sends
  // `set_craft_blocked` with the whole effective list; Settings is never
  // changed from here.
  import type { Send } from '../bridge';
  import type { CatalogueGroup, CraftBlockedView, PreviewView } from '../ipc';
  import CurrencyPicker from '../components/CurrencyPicker.svelte';

  let {
    blocked,
    preview,
    catalogue,
    send,
  }: { blocked: CraftBlockedView; preview: PreviewView | null; catalogue: CatalogueGroup[]; send: Send } = $props();

  const FROM_WORDS = { settings: 'Settings', craft: 'this craft' } as const;

  /** The player's own choice, once made; until then the view decides. */
  let chosenOpen = $state<boolean | null>(null);

  const likelyFix = $derived(
    preview !== null && (preview.route === null || preview.route.steps.some((step) => step.fallback !== null)),
  );
  const open = $derived(chosenOpen ?? likelyFix);
  const selected = $derived(blocked.blocked.map((entry) => entry.id));
  const tags = $derived<Record<string, string>>({
    ...Object.fromEntries(blocked.allowed.map((entry) => [entry.id, 'allowed for this craft'])),
    ...Object.fromEntries(blocked.blocked.map((entry) => [entry.id, FROM_WORDS[entry.from]])),
  });

  /** Only a change the page did not make itself is the player's. */
  function ontoggle(event: Event): void {
    const details = event.currentTarget as HTMLDetailsElement;
    if (details.open !== open) chosenOpen = details.open;
  }
</script>

<details class="blocked" {open} {ontoggle}>
  <summary><span class="caps">Blocked for this craft: {blocked.blocked.length}</span></summary>
  <div class="body">
    <p class="help">Changes here apply to this craft only; Settings keeps your usual list.</p>
    <CurrencyPicker groups={catalogue} {selected} {tags} onchange={(ids) => send('set_craft_blocked', { blocked: ids })} />
  </div>
</details>

<style>
  .blocked {
    border: var(--border-width) solid var(--color-panel-border);
    background: var(--color-panel-raised);
  }

  summary {
    padding: calc(var(--space-unit) * 2) calc(var(--space-unit) * 3);
    color: var(--color-text-primary);
    font-size: var(--size-step);
    line-height: var(--line-step);
    cursor: pointer;
  }

  summary:hover {
    background: var(--color-action-secondary-hover);
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: calc(var(--space-unit) * 2);
    padding: 0 calc(var(--space-unit) * 3) calc(var(--space-unit) * 3);
  }

  .help {
    margin: 0;
    color: var(--color-text-muted);
    font-size: var(--size-step);
    line-height: var(--line-step);
  }
</style>
