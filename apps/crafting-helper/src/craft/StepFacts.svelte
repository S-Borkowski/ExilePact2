<script lang="ts">
  // A step's facts under its instruction, as the preview and the guide's
  // card show them: the "Targets" line (the target modifiers a hit of the
  // step may place, as Rust sent them; none for a step that places none),
  // the "Uses" line (currency, then omens), the odds as
  // Rust wrote them (`odds unknown: <reason>` kept whole, never a blank or a
  // dash), and the fallback line when an earlier alternative was passed
  // over. A manual step has neither uses nor odds, so shows no such line.
  import type { StepLineView } from '../ipc';

  let { step }: { step: StepLineView } = $props();
</script>

{#if step.targets.length > 0}
  <p class="step-targets">
    <span class="label">Targets</span>
    {#each step.targets as target, i (i)}{#if i > 0}<span class="sep" aria-hidden="true"> · </span>{/if}<span class="target">{target}</span>{/each}
  </p>
{/if}
{#if step.uses.length > 0}
  <p class="uses"><span class="label">Uses</span> {step.uses.join(', ')}</p>
{/if}
{#if step.odds !== null}
  <p class="odds" data-known={step.odds.startsWith('odds unknown') ? 'false' : 'true'}>{step.odds}</p>
{/if}
{#if step.fallback !== null}
  <p class="fallback"><span class="label caps">Fallback</span> <span class="line">{step.fallback}</span></p>
{/if}

<style>
  p {
    margin: 0;
    font-size: var(--size-step);
    line-height: var(--line-step);
    color: var(--color-text-secondary);
  }

  .label {
    color: var(--color-text-muted);
  }

  .step-targets .target {
    color: var(--color-text-primary);
  }

  .step-targets .sep {
    color: var(--color-text-muted);
  }

  .odds {
    font-variant-numeric: tabular-nums;
  }

  /* Unknown odds are said in words; the muted colour only repeats it. */
  .odds[data-known='false'] {
    color: var(--color-text-muted);
  }

  .fallback .label {
    font-size: var(--size-caption);
    letter-spacing: 0.03em;
  }

  .fallback .line {
    color: var(--color-status-in-progress);
  }
</style>
