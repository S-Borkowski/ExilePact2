<script lang="ts">
  // The Settings window's root (`settings.html`): the title, each section of
  // `SECTIONS` in order under its heading, and the credit. It renders the
  // last `SettingsView` the bridge delivered; until the first arrives it
  // shows the title and the credit only.
  import { onDestroy, onMount } from 'svelte';
  import type { SettingsBridge, SettingsSend } from './bridge';
  import { SECTIONS } from './sections';
  import type { SettingsView } from './view';

  let { bridge }: { bridge: SettingsBridge } = $props();

  let view = $state<SettingsView | null>(null);
  let stop: (() => void) | null = null;

  const send: SettingsSend = (name, args) => {
    void bridge.command(name, args);
  };

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
        (error) => console.error('crafting-helper settings: the bridge did not start', error),
      );
    return () => {
      live = false;
    };
  });

  onDestroy(() => {
    stop?.();
  });
</script>

<div class="window" data-state={view?.update.state ?? 'none'} data-install={view?.install.state ?? 'none'}>
  <main aria-labelledby="settings-title">
    <h1 class="title caps" id="settings-title">Settings</h1>
    {#if view}
      {#each SECTIONS as section (section.id)}
        <section class="section" aria-labelledby="section-{section.id}">
          <h2 class="group-label" id="section-{section.id}">{section.title}</h2>
          <section.component {view} {send} />
        </section>
      {/each}
    {/if}
  </main>
  <footer class="credit caption">Created by ST4H</footer>
</div>

<style>
  /* As tall as its content: the shell sizes the window to its tallest
     state, and a taller window leaves its spare height below the credit,
     not inside the page. */
  .window {
    color: var(--color-text-primary);
    font-size: var(--size-step);
    line-height: var(--line-step);
    font-kerning: normal;
    -webkit-font-smoothing: antialiased;
    user-select: none;
  }

  .title {
    margin: 0;
    padding: calc(var(--space-unit) * 4) calc(var(--space-unit) * 5) var(--space-unit);
    font-size: 18px;
    line-height: 24px;
  }

  .section {
    padding: 0 calc(var(--space-unit) * 5);
  }

  .section + .section {
    border-top: var(--border-width) solid var(--color-panel-divider);
  }

  .credit {
    padding: calc(var(--space-unit) * 3) calc(var(--space-unit) * 5);
    border-top: var(--border-width) solid var(--color-panel-divider);
  }
</style>
