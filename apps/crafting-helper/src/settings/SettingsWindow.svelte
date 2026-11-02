<script lang="ts">
  // The Settings window's root (`settings.html`): the title, each section of
  // `SECTIONS` in order under its heading, and the credit. It renders the
  // last `SettingsView` the bridge delivered; until the first arrives it
  // shows the title and the credit only. The catalogue the blocked
  // currencies are picked from is read once, at the start.
  //
  // The view's theme marks the document (`../theme.ts`), Exile until the
  // first view arrives. Exile draws the steel/masonry kit's frame over the
  // window and its masonry behind the content (`../steel.css`): the title
  // and the credit sit between the frame's corner knots and the sections
  // scroll between them, inside the rails. Claude draws today's window.
  // The Appearance section carries `data-theme-setting`, which the legacy
  // pixel comparison removes (the base tag has no such section).
  import { onDestroy, onMount } from 'svelte';
  import type { CatalogueGroup } from '../ipc';
  import type { SettingsSend, SettingsWindowBridge } from './bridge';
  import { SECTIONS } from './sections';
  import type { SettingsView } from './view';
  import MaterialFrame from '../components/MaterialFrame.svelte';
  import masonry from '../../theme/materials/raster/masonry-charcoal.png';
  import { applyTheme } from '../theme';

  let { bridge }: { bridge: SettingsWindowBridge } = $props();

  let view = $state<SettingsView | null>(null);
  let catalogue = $state<CatalogueGroup[]>([]);
  let stop: (() => void) | null = null;
  const exile = $derived(view?.theme !== 'claude');

  $effect(() => {
    applyTheme(document.documentElement, view?.theme);
  });

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
    bridge.catalogue().then(
      (groups) => {
        if (live) catalogue = groups;
      },
      (error) => console.error('crafting-helper settings: the catalogue did not load', error),
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
    <h1 class="title caps" id="settings-title">Settings</h1>
    <div class="window-scroll">
      {#if view}
        {#each SECTIONS as section (section.id)}
          <section
            class="section"
            aria-labelledby="section-{section.id}"
            data-theme-setting={section.id === 'appearance' ? '' : undefined}
          >
            <h2 class="group-label" id="section-{section.id}">{section.title}</h2>
            <section.component {view} {send} {catalogue} />
          </section>
        {/each}
      {/if}
    </div>
  </main>
  <footer class="credit caption">Created by ST4H</footer>
{/snippet}

<div
  class="window"
  data-state={view?.update.state ?? 'none'}
  data-install={view?.install.state ?? 'none'}
  style:--well-image={exile ? `url("${masonry}")` : null}
>
  {#if exile}
    <MaterialFrame />
    <div class="window-content">{@render windowContents()}</div>
  {:else}
    {@render windowContents()}
  {/if}
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
