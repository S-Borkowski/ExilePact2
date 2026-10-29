<script lang="ts">
  // Craft mode refused (`ui-design.md` § "Mode refused"): the last grab is
  // not an item the engine plans for. The reason as Rust sent it, how to
  // grab another item, and the grabbed text, open.
  import type { GrabView } from '../ipc';
  import GrabSteps from '../components/GrabSteps.svelte';
  import GrabbedText from '../components/GrabbedText.svelte';

  let { refusal, grab, hotkey }: { refusal: string; grab: GrabView | null; hotkey: string } = $props();

  const uid = $props.id();
</script>

<section class="refused" aria-labelledby="{uid}-title">
  <h2 class="refused-title" id="{uid}-title" tabindex="-1" data-mode-heading>Can't plan this item</h2>
  <p class="refusal">{refusal}</p>
  <div class="grab-again">
    <p>Grab another item:</p>
    <GrabSteps {hotkey} />
  </div>
  {#if grab}
    <GrabbedText {grab} open />
  {/if}
</section>

<style>
  .refused {
    display: flex;
    flex-direction: column;
    gap: var(--space-section-gap);
    max-width: 760px;
  }

  .refused-title {
    margin: 0;
    color: var(--color-text-primary);
    font-size: var(--size-title);
    line-height: var(--line-title);
    font-weight: 700;
  }

  .refusal {
    margin: 0;
    color: var(--color-text-primary);
    font-size: var(--size-row);
    line-height: var(--line-row);
  }

  .grab-again {
    color: var(--color-text-secondary);
    font-size: var(--size-step);
    line-height: var(--line-step);
  }

  .grab-again p {
    margin: 0 0 var(--space-unit);
  }
</style>
