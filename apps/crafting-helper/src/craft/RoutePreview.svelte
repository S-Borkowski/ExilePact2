<script lang="ts">
  // The route preview (`ui-design.md` § "Mode planning"): nothing but a
  // prompt with no target; with a route, its source, the route select when
  // other routes apply (every route as `Route <n>`, its number from Rust,
  // in Rust's order, the previewed one selected where it stands, so a
  // route keeps its number when chosen), the chosen targets with their floors, and the
  // numbered steps, each naming its target modifiers; with no route, one
  // reason per rejected blueprint in Rust's order. No blueprint's name: the
  // route is the app's recipe, not the player's. What the craft targets
  // and what to do, never the reasoning behind the technique. The route the
  // engine previews, the floor labels, and why a route does not apply, are
  // Rust's.
  import type { Send } from '../bridge';
  import type { ChosenTarget, PlanningView } from '../ipc';
  import StepFacts from './StepFacts.svelte';

  let { planning, send }: { planning: PlanningView; send: Send } = $props();

  const uid = $props.id();
  const preview = $derived(planning.preview);
  /** Each choice's floor options, to say a target's floor in Rust's words. */
  const floorsOf = $derived(
    new Map(
      [...planning.choices.prefixes, ...planning.choices.suffixes, ...planning.choices.desecrated].map((c) => [c.mod_id, c.floors]),
    ),
  );
  const floorLabel = (target: ChosenTarget): string | null =>
    floorsOf.get(target.mod_id)?.find((option) => option.floor === target.floor)?.label ?? null;

  function chooseRoute(event: Event, current: string): void {
    const select = event.currentTarget as HTMLSelectElement;
    const blueprint = select.value;
    select.value = current;
    if (blueprint !== current) send('choose_route', { blueprint });
  }
</script>

<section class="route" aria-labelledby="{uid}-title">
  <h2 class="section-title caps" id="{uid}-title">Route</h2>
  {#if preview === null}
    <p class="no-target">Pick a target to see a route.</p>
  {:else if preview.route}
    {@const route = preview.route}
    {#if preview.routes.length > 1}
      <div class="head">
        <select class="field" aria-label="Route" value={route.blueprint} onchange={(event) => chooseRoute(event, route.blueprint)}>
          {#each preview.routes as choice (choice.blueprint)}
            <option value={choice.blueprint}>Route {choice.number}</option>
          {/each}
        </select>
      </div>
    {/if}
    <p class="route-source">{route.source}</p>
    {#if planning.targets.length > 0}
      <h3 class="section-title caps" id="{uid}-targets">Targets</h3>
      <ul class="route-targets" aria-labelledby="{uid}-targets">
        {#each planning.targets as target (target.mod_id)}
          {@const floor = floorLabel(target)}
          <li><span class="target-text">{target.text}</span>{#if floor !== null}<span class="floor">{floor}</span>{/if}</li>
        {/each}
      </ul>
    {/if}
    <ol class="steps">
      {#each route.steps as step (step.id)}
        <li>
          <span class="number" aria-hidden="true">{step.number}</span>
          <div class="step-body">
            <p class="instruction">{step.instruction}</p>
            <StepFacts {step} />
          </div>
        </li>
      {/each}
    </ol>
  {:else}
    <p class="no-route">No route reaches these targets.</p>
    <ul class="rejected">
      {#each preview.rejected as line (line.blueprint)}
        <li>{line.reason}</li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .route {
    display: flex;
    flex-direction: column;
    gap: calc(var(--space-unit) * 2);
  }

  p {
    margin: 0;
    font-size: var(--size-step);
    line-height: var(--line-step);
  }

  .no-target {
    color: var(--color-text-muted);
  }

  .head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: calc(var(--space-unit) * 2);
  }

  .route-source {
    color: var(--color-text-muted);
  }

  .route-targets li {
    display: flex;
    flex-wrap: wrap;
    justify-content: space-between;
    gap: 0 calc(var(--space-unit) * 3);
    padding: var(--space-unit) 0;
    font-size: var(--size-step);
    line-height: var(--line-step);
  }

  .route-targets .target-text {
    color: var(--color-text-primary);
  }

  .route-targets .floor {
    color: var(--color-text-secondary);
    font-variant-numeric: tabular-nums;
  }

  ol,
  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .steps li {
    display: flex;
    gap: calc(var(--space-unit) * 3);
    padding: calc(var(--space-unit) * 2) 0;
    border-top: var(--border-width) solid var(--color-panel-divider);
  }

  /* The step's number, from the view; the ordered list says it to a screen reader. */
  .number {
    flex: none;
    width: 22px;
    height: 22px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border: var(--border-width) solid var(--color-panel-border);
    color: var(--color-text-secondary);
    font-size: var(--size-caption);
    font-variant-numeric: tabular-nums;
  }

  .step-body {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .instruction {
    color: var(--color-text-primary);
  }

  .no-route {
    color: var(--color-text-primary);
    font-weight: 700;
  }

  .rejected li {
    padding: var(--space-unit) 0;
    border-top: var(--border-width) solid var(--color-panel-divider);
    color: var(--color-text-secondary);
    font-size: var(--size-step);
    line-height: var(--line-step);
  }
</style>
