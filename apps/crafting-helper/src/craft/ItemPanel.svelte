<script lang="ts">
  // The parsed item, as planning shows it and guiding shows the item now:
  // its name, one line of facts, the explicit modifiers by side with their
  // tier and tags in words, then the grabbed text, closed. Every value is
  // the view's; "target" is Rust's judgement (`meets_target`), not the page's.
  import type { CraftItemView, GrabView, ItemModView } from '../ipc';
  import GrabbedText from '../components/GrabbedText.svelte';

  let {
    item,
    grab,
    modeHeading = false,
  }: {
    item: CraftItemView;
    grab: GrabView | null;
    /** This panel's heading is the mode's first, which takes the focus when the mode changes. */
    modeHeading?: boolean;
  } = $props();

  const uid = $props.id();

  const title = $derived(item.name ?? item.base_type ?? item.class);
  const facts = $derived(
    [item.rarity, item.base_type, item.class, item.item_level === null ? null : `item level ${item.item_level}`]
      .filter((fact): fact is string => fact !== null && fact !== '')
      .join(' · '),
  );
  const sides = $derived(
    [
      { side: 'prefix', label: 'Prefixes', mods: item.modifiers.filter((m) => m.side === 'prefix') },
      { side: 'suffix', label: 'Suffixes', mods: item.modifiers.filter((m) => m.side === 'suffix') },
    ].filter((group) => group.mods.length > 0),
  );

  const tags = (mod: ItemModView): string[] =>
    [mod.fractured && 'fractured', mod.desecrated && 'desecrated', mod.unrevealed && 'unrevealed'].filter(
      (tag): tag is string => typeof tag === 'string',
    );
</script>

<section class="item-panel" aria-labelledby="{uid}-title">
  <h2 class="item-title" id="{uid}-title" tabindex="-1" data-mode-heading={modeHeading ? '' : undefined}>
    {title}
  </h2>
  <p class="facts">{facts}</p>

  {#each sides as group (group.side)}
    <h3 class="section-title caps" id="{uid}-{group.side}">{group.label}</h3>
    <ul class="mods" data-side={group.side} aria-labelledby="{uid}-{group.side}">
      {#each group.mods as mod, i (i)}
        <li class:meets={mod.meets_target}>
          <div class="text">
            {#each mod.lines as line, j (j)}<span class="line">{line}</span>{/each}
            {#if tags(mod).length > 0 || mod.meets_target}
              <span class="tags">
                {#each tags(mod) as tag (tag)}<span class="tag">{tag}</span>{/each}
                {#if mod.meets_target}<span class="tag target"><span class="check" aria-hidden="true"></span>target</span>{/if}
              </span>
            {/if}
          </div>
          {#if mod.tier === null}
            <span class="tier none" aria-label="no tier">—</span>
          {:else}
            <span class="tier">T{mod.tier}</span>
          {/if}
        </li>
      {/each}
    </ul>
  {:else}
    <p class="no-mods">No explicit modifiers.</p>
  {/each}

  {#if grab}
    <GrabbedText {grab} />
  {/if}
</section>

<style>
  .item-panel {
    display: flex;
    flex-direction: column;
    gap: calc(var(--space-unit) * 2);
  }

  .item-title {
    margin: 0;
    color: var(--color-text-primary);
    font-size: var(--size-title);
    line-height: var(--line-title);
    font-weight: 700;
  }

  .facts {
    margin: 0;
    color: var(--color-text-secondary);
    font-size: var(--size-step);
    line-height: var(--line-step);
  }

  .mods {
    margin: 0;
    padding: 0;
    list-style: none;
    border-top: var(--border-width) solid var(--color-panel-divider);
  }

  .mods li {
    display: flex;
    align-items: baseline;
    gap: calc(var(--space-unit) * 3);
    padding: calc(var(--space-unit) * 1.5) 0;
    border-bottom: var(--border-width) solid var(--color-panel-divider);
  }

  /* A hybrid modifier's lines stay together, one under the other. */
  .text {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
  }

  .line {
    color: var(--color-text-primary);
    font-size: var(--size-step);
    line-height: var(--line-step);
  }

  .tags {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-unit);
    margin-top: 2px;
  }

  .tier {
    flex: none;
    color: var(--color-text-secondary);
    font-size: var(--size-step);
    font-variant-numeric: tabular-nums;
  }

  .meets .tier {
    color: var(--want);
    font-weight: 700;
  }

  .target {
    color: var(--want);
    border-color: var(--want);
  }

  /* A check mark drawn from two borders, so the tag is not told by colour alone. */
  .check {
    display: inline-block;
    width: 4px;
    height: 8px;
    margin-right: 4px;
    border: solid currentColor;
    border-width: 0 2px 2px 0;
    transform: translateY(-1px) rotate(45deg);
  }

  .no-mods {
    margin: 0;
    color: var(--color-text-muted);
    font-size: var(--size-step);
  }
</style>
