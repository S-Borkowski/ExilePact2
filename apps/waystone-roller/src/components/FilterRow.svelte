<script lang="ts">
  // The filter row: tier range, rarities, corruption and the want mode. Each
  // change sends only the keys it changes; the control then shows the view's
  // value again until the shell's next view arrives with the new one, so a
  // refused change never leaves a control showing what the shell did not take.
  import type { Send } from '../bridge';
  import { CORRUPTIONS, type Corruption, type Rarity, type Selection, TIER_MAX, TIER_MIN, WANT_MODES, type WantMode } from '../ipc';
  import { restoreGroup } from './controls';

  let { selection, send }: { selection: Selection; send: Send } = $props();

  const range = (from: number, to: number) => Array.from({ length: to - from + 1 }, (_, i) => from + i);

  const RARITIES: Array<{ key: keyof Rarity; label: string }> = [
    { key: 'normal', label: 'Normal' },
    { key: 'magic', label: 'Magic' },
    { key: 'rare', label: 'Rare' },
  ];
  const CORRUPTION_LABELS: Record<Corruption, string> = {
    any: 'Any',
    corrupted: 'Corrupted',
    uncorrupted: 'Not corrupted',
  };
  const WANT_MODE_LABELS: Record<WantMode, string> = { any: 'Any', all: 'All' };
  const WANT_MODE_CAPTIONS: Record<WantMode, string> = {
    any: 'any one is enough',
    all: 'all must be there',
  };

  function onTier(key: 'tier_min' | 'tier_max', event: Event & { currentTarget: HTMLSelectElement }): void {
    const value = Number(event.currentTarget.value);
    event.currentTarget.value = String(selection[key]);
    if (value !== selection[key]) send('update_selection', { patch: { [key]: value } });
  }

  function toggleRarity(key: keyof Rarity): void {
    send('update_selection', { patch: { rarity: { ...selection.rarity, [key]: !selection.rarity[key] } } });
  }

  function onCorruption(value: Corruption, event: Event & { currentTarget: HTMLInputElement }): void {
    restoreGroup(event.currentTarget, selection.corruption);
    send('update_selection', { patch: { corruption: value } });
  }

  function onWantMode(value: WantMode, event: Event & { currentTarget: HTMLInputElement }): void {
    restoreGroup(event.currentTarget, selection.want_mode);
    send('update_selection', { patch: { want_mode: value } });
  }
</script>

<section class="filters" aria-label="Rules">
  <div class="filter">
    <span class="filter-label caps" id="tier-label">Tier</span>
    <div class="tier" role="group" aria-labelledby="tier-label">
      <label class="tier-part">
        <span>from</span>
        <select class="field tier-min" value={selection.tier_min} onchange={(e) => onTier('tier_min', e)}>
          {#each range(TIER_MIN, selection.tier_max) as n (n)}
            <option value={n} selected={n === selection.tier_min}>{n}</option>
          {/each}
        </select>
      </label>
      <label class="tier-part">
        <span>to</span>
        <select class="field tier-max" value={selection.tier_max} onchange={(e) => onTier('tier_max', e)}>
          {#each range(selection.tier_min, TIER_MAX) as n (n)}
            <option value={n} selected={n === selection.tier_max}>{n}</option>
          {/each}
        </select>
      </label>
    </div>
  </div>

  <div class="filter">
    <span class="filter-label caps" id="rarity-label">Rarity</span>
    <div class="toggles" role="group" aria-labelledby="rarity-label">
      {#each RARITIES as r (r.key)}
        <button
          type="button"
          class="btn toggle"
          data-rarity={r.key}
          aria-pressed={selection.rarity[r.key]}
          onclick={() => toggleRarity(r.key)}
        >
          <svg class="check" viewBox="0 0 12 12" aria-hidden="true" focusable="false">
            {#if selection.rarity[r.key]}
              <path d="M2 6.5 4.8 9 10 3" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="square" />
            {:else}
              <rect x="1.5" y="1.5" width="9" height="9" fill="none" stroke="currentColor" stroke-width="1" />
            {/if}
          </svg>
          {r.label}
        </button>
      {/each}
    </div>
  </div>

  <div class="filter">
    <span class="filter-label caps" id="corruption-label">Corruption</span>
    <div class="segmented" role="radiogroup" aria-labelledby="corruption-label">
      {#each CORRUPTIONS as c (c)}
        <label>
          <input
            type="radio"
            name="corruption"
            value={c}
            checked={selection.corruption === c}
            onchange={(e) => onCorruption(c, e)}
          />
          {CORRUPTION_LABELS[c]}
        </label>
      {/each}
    </div>
  </div>

  <div class="filter">
    <span class="filter-label caps" id="want-mode-label">Wanted mods</span>
    <div class="want-mode">
      <div class="segmented" role="radiogroup" aria-labelledby="want-mode-label" aria-describedby="want-mode-caption">
        {#each WANT_MODES as m (m)}
          <label>
            <input
              type="radio"
              name="want-mode"
              value={m}
              checked={selection.want_mode === m}
              onchange={(e) => onWantMode(m, e)}
            />
            {WANT_MODE_LABELS[m]}
          </label>
        {/each}
      </div>
      <span class="caption" id="want-mode-caption">{WANT_MODE_CAPTIONS[selection.want_mode]}</span>
    </div>
  </div>
</section>

<style>
  .filters {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-start;
    gap: calc(var(--space-unit) * 3) calc(var(--space-unit) * 5);
    padding: calc(var(--space-unit) * 3) calc(var(--space-unit) * 5) calc(var(--space-unit) * 2);
    background: var(--color-panel-raised);
    border-top: var(--border-width) solid var(--color-panel-divider);
    border-bottom: var(--border-width) solid var(--color-panel-divider);
  }

  .filter {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: calc(var(--space-unit) * 1.5);
  }

  .filter-label {
    color: var(--color-text-secondary);
    font-size: var(--size-label);
    line-height: 16px;
  }

  .tier {
    display: flex;
    gap: calc(var(--space-unit) * 2);
  }

  .tier-part {
    display: inline-flex;
    align-items: center;
    gap: calc(var(--space-unit) * 1.5);
    color: var(--color-text-muted);
    font-size: var(--size-label);
  }

  .tier select {
    width: 52px;
    font-variant-numeric: tabular-nums;
  }

  .toggles {
    display: flex;
    gap: var(--space-unit);
  }

  .toggle {
    padding: 0 calc(var(--space-unit) * 2.5);
  }

  .toggle[aria-pressed='true'] {
    background: var(--color-text-secondary);
    color: var(--color-text-on-light);
    border-color: var(--color-text-secondary);
    font-weight: 700;
  }

  .toggle[aria-pressed='true']:hover {
    background: var(--color-text-primary);
    border-color: var(--color-text-primary);
  }

  .check {
    width: 12px;
    height: 12px;
    flex: none;
  }

  .want-mode {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
  }

  .caption {
    white-space: nowrap;
    color: var(--color-text-muted);
    font-size: var(--size-caption);
    line-height: var(--line-caption);
  }
</style>
