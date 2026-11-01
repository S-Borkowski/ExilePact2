<script lang="ts">
  // The Settings window's root (`settings.html`): the title, each section of
  // `SECTIONS` in order under its heading, and the credit. It renders the
  // last `AppSettingsView` the bridge delivered; until the first arrives it
  // shows the title and the credit only.
  import { onDestroy, onMount } from 'svelte';
  import type { SettingsBridge, SettingsSend } from './bridge';
  import { SECTIONS } from './sections';
  import MaterialFrame from '../components/MaterialFrame.svelte';
  import stone from '../../theme/materials/raster/charcoal-stone-well.png';
  import { applyPresentationTheme } from '../theme';
  import type { AppSettingsView } from './view';

  let { bridge }: { bridge: SettingsBridge } = $props();

  let view = $state<AppSettingsView | null>(null);
  let stop: (() => void) | null = null;
  $effect(() => { applyPresentationTheme(document.documentElement, view?.theme ?? 'exile'); });

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
        (error) => console.error('quest-tracker settings: the bridge did not start', error),
      );
    return () => {
      live = false;
    };
  });

  onDestroy(() => {
    stop?.();
  });
</script>

{#snippet windowContents()}
  <main aria-labelledby="settings-title">
    <h1 class="title" id="settings-title">Settings</h1>
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
{/snippet}

<div class="window" style:--well-image={view?.theme !== 'claude' ? `url("${stone}")` : null} data-state={view?.update.state ?? 'none'} data-theme={view?.theme ?? 'exile'}>
  {#if view?.theme !== 'claude'}
    <MaterialFrame plaque />
    <div class="window-content">{@render windowContents()}</div>
  {:else}
    {@render windowContents()}
  {/if}
</div>

<style>
  /* An ordinary window: opaque, dark, under the system's dark title bar. */
  :global(html),
  :global(body) {
    background: var(--color-panel-bg);
    color-scheme: dark;
  }

  /* As tall as its content: the shell sizes the window to it, and a taller
     window leaves its spare height below the credit, not inside the page. */
  .window {
    color: var(--color-text-primary);
    font-family: var(--font-family);
    font-size: var(--size-step);
    font-weight: var(--font-weight-row);
    line-height: var(--line-step);
    font-kerning: normal;
    -webkit-font-smoothing: antialiased;
    user-select: none;
  }

  .title {
    margin: 0;
    padding: 16px 16px var(--space-unit);
    font-size: var(--size-title);
    font-weight: var(--font-weight-title);
    line-height: var(--line-title);
  }

  .section {
    padding: 0 16px;
  }

  .section + .section {
    border-top: var(--border-width) solid var(--color-panel-divider);
  }

  /* The group label keeps its own vertical rhythm; the section gives the sides. */
  .section .group-label {
    padding-right: 0;
    padding-left: 0;
  }

  .credit {
    padding: 12px 16px;
    border-top: var(--border-width) solid var(--color-panel-divider);
  }
</style>
