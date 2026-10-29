<script lang="ts">
  // Craft mode none, nothing grabbed: M3's empty state, unchanged. How to
  // grab: the game's own Ctrl+Alt+C, then the app's hotkey or Ctrl+V here.
  import type { View } from '../ipc';
  import GrabSteps from './GrabSteps.svelte';
  import GrabbedText from './GrabbedText.svelte';

  let { view }: { view: View } = $props();
</script>

<section class="panel" aria-labelledby="grab-title">
  <h2 class="section-title caps" id="grab-title" tabindex="-1" data-mode-heading>Last grabbed item</h2>
  <div class="howto">
    <p>Nothing grabbed yet. To grab an item:</p>
    <GrabSteps hotkey={view.hotkey} />
  </div>
  {#if view.grab}
    <GrabbedText grab={view.grab} open />
  {/if}
</section>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    gap: var(--space-section-gap);
  }

  .howto {
    color: var(--color-text-secondary);
    font-size: var(--size-step);
    line-height: var(--line-step);
  }

  .howto p {
    margin: 0 0 var(--space-unit);
  }
</style>
