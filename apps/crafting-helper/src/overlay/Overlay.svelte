<script lang="ts">
  // The overlay's root (`overlay.html`): one line, `view.overlay.line`, from
  // the last `View` the bridge delivered. The window is click-through and
  // never takes focus; the shell shows it only while a craft is active
  // (`view.overlay.visible`) and hides it otherwise, so the page draws the
  // line in both cases and sends no command.
  import { onDestroy, onMount } from 'svelte';
  import type { Bridge } from '../bridge';
  import type { View } from '../ipc';

  let { bridge }: { bridge: Bridge } = $props();

  let view = $state<View | null>(null);
  let stop: (() => void) | null = null;

  onMount(() => {
    let live = true;
    bridge
      .start((next) => {
        view = next;
      })
      .then(
        (unsubscribe) => {
          if (live) stop = unsubscribe;
          else unsubscribe();
        },
        (error) => console.error('crafting-helper overlay: the bridge did not start', error),
      );
    return () => {
      live = false;
    };
  });

  onDestroy(() => {
    stop?.();
  });
</script>

<div class="overlay" data-visible={view ? String(view.overlay.visible) : 'none'}>
  {#if view}
    <p class="line">{view.overlay.line}</p>
  {/if}
</div>

<style>
  .overlay {
    display: flex;
    align-items: flex-start;
    height: 100%;
    padding: calc(var(--space-unit) * 2);
  }

  /* The theme's panel, slightly see-through, so the game shows round it. */
  .line {
    max-width: 100%;
    margin: 0;
    padding: calc(var(--space-unit) * 1.5) calc(var(--space-unit) * 3);
    border: var(--border-width) solid var(--color-panel-border);
    background: color-mix(in srgb, var(--color-panel-bg) calc(var(--opacity-default) * 100%), transparent);
    box-shadow: var(--shadow-panel);
    color: var(--color-text-primary);
    font-size: var(--size-row);
    line-height: var(--line-row);
    font-weight: var(--font-weight-row);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
