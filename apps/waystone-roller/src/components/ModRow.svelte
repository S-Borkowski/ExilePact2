<script lang="ts">
  // One mod: its lines, tier count and token, and the three-way choice
  // Want / — / Avoid. A mod without a token of its own cannot be wanted (the
  // shell says why in `why_not`); it can still be avoided.
  import type { Send } from '../bridge';
  import type { ModView, Selection } from '../ipc';
  import { type Choice, choiceOf, choicePatch, plural } from '../selection';
  import { restoreGroup } from './controls';

  let { mod, selection, unmatchable, send }: { mod: ModView; selection: Selection; unmatchable: boolean; send: Send } =
    $props();

  const current = $derived(choiceOf(selection, mod.id));
  const CHOICES: Array<{ value: Choice; text: string; name: string }> = [
    { value: 'want', text: 'Want', name: 'Want' },
    { value: 'none', text: '—', name: 'Neither' },
    { value: 'avoid', text: 'Avoid', name: 'Avoid' },
  ];
  const MARKS: Record<Choice, string> = { want: '+', none: '', avoid: '−' };

  function pick(to: Choice, event: Event & { currentTarget: HTMLInputElement }): void {
    restoreGroup(event.currentTarget, current);
    if (to !== current) send('update_selection', { patch: choicePatch(selection, mod.id, to) });
  }
</script>

<li class="mod" data-mod-id={mod.id} data-choice={current} data-unmatchable={unmatchable}>
  <span class="choice-mark" aria-hidden="true">{MARKS[current]}</span>
  <div class="text">
    <span class="label">{mod.label}</span>
    {#each mod.lines.slice(1) as line, i (i)}
      <span class="line">{line}</span>
    {/each}
    {#if mod.name !== null}
      <span class="affix">{mod.name} <span class="kind">({mod.kind})</span></span>
    {/if}
    {#if mod.why_not !== null}
      <span class="why-not">{mod.why_not}</span>
    {/if}
    {#if unmatchable}
      <span class="unmatchable-mark"><span class="bang" aria-hidden="true">!</span>Cannot be matched</span>
    {/if}
  </div>
  <div class="meta">
    <span class="tiers">{plural(mod.tiers, 'tier', 'tiers')}</span>
    {#if mod.token !== null}
      <code class="token" title="The text the search uses for this mod">{mod.token}</code>
    {:else}
      <span class="no-token">no token</span>
    {/if}
  </div>
  <div class="segmented choices" role="radiogroup" aria-label={mod.label}>
    {#each CHOICES as c (c.value)}
      <label class="choice-{c.value}" title={c.value === 'want' && mod.token === null ? 'Cannot be wanted on its own' : c.name}>
        <input
          type="radio"
          name="choice-{mod.id}"
          value={c.value}
          aria-label={c.name}
          checked={current === c.value}
          disabled={c.value === 'want' && mod.token === null}
          onchange={(e) => pick(c.value, e)}
        />
        {c.text}
      </label>
    {/each}
  </div>
</li>

<style>
  .mod {
    display: grid;
    grid-template-columns: 16px minmax(0, 1fr) auto auto;
    align-items: center;
    gap: calc(var(--space-unit) * 3);
    min-height: var(--size-row-height);
    padding: calc(var(--space-unit) * 2) 0;
    border-bottom: var(--border-width) solid var(--color-panel-divider);
  }

  .mod[data-choice='want'] {
    background: linear-gradient(to right, color-mix(in srgb, var(--want) 8%, transparent), transparent 60%);
  }

  .mod[data-choice='avoid'] {
    background: linear-gradient(to right, color-mix(in srgb, var(--avoid) 8%, transparent), transparent 60%);
  }

  .choice-mark {
    justify-self: center;
    font-size: 18px;
    font-weight: 700;
    line-height: 1;
  }

  .mod[data-choice='want'] .choice-mark {
    color: var(--want);
  }

  .mod[data-choice='avoid'] .choice-mark {
    color: var(--avoid);
  }

  .text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .label {
    color: var(--color-text-primary);
    font-weight: 500;
  }

  .line,
  .affix {
    color: var(--color-text-secondary);
    font-size: var(--size-step);
    line-height: var(--line-step);
  }

  .kind {
    color: var(--color-text-muted);
  }

  .why-not {
    color: var(--color-text-muted);
    font-size: var(--size-caption);
    line-height: var(--line-caption);
    font-style: italic;
  }

  .unmatchable-mark {
    color: var(--error);
    font-size: var(--size-caption);
    line-height: var(--line-caption);
    font-weight: 700;
  }

  .bang {
    margin-right: 0.35em;
  }

  .meta {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 2px;
    color: var(--color-text-muted);
    font-size: var(--size-caption);
    line-height: var(--line-caption);
    white-space: nowrap;
  }

  .token {
    padding: 0 var(--space-unit);
    background: var(--color-panel-sunken);
    color: var(--color-text-secondary);
    font-family: var(--mono);
    font-size: 12px;
  }

  .no-token {
    font-style: italic;
  }

  .choices label {
    min-width: 52px;
  }

  .choices label.choice-none {
    min-width: 32px;
  }

  .choices label.choice-want:has(input:checked) {
    background: var(--want);
  }

  .choices label.choice-avoid:has(input:checked) {
    background: var(--avoid);
  }

  @media (max-width: 620px) {
    .mod {
      grid-template-columns: 16px minmax(0, 1fr) auto;
    }

    .meta {
      grid-column: 2;
      grid-row: 2;
      flex-direction: row;
      align-items: center;
      gap: calc(var(--space-unit) * 2);
    }

    .choices {
      grid-column: 3;
      grid-row: 1 / span 2;
    }
  }
</style>
