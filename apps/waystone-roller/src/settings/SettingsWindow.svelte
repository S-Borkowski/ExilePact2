<script lang="ts">
  // The Settings window's root (`settings.html`): the title, each section of
  // `SECTIONS` in order under its heading, and the credit. It renders the
  // last `AppSettingsView` the bridge delivered; until the first arrives it
  // shows the title and the credit only.
  //
  // Exile draws the steel/masonry kit's neutral frame over a masonry well
  // (`../steel.css`), its art meeting the window's edge as the Quest
  // Tracker's Settings window does: the title and the credit sit between the
  // frame's corner knots, and the sections scroll between them, inside the
  // rails, so no line or control ever crosses the artwork. Claude draws the
  // window as it was.
  import { onDestroy, onMount } from 'svelte';
  import MaterialFrame from '../components/MaterialFrame.svelte';
  import masonry from '../../theme/materials/raster/masonry-charcoal.png';
  import { applyTheme, themeOf } from '../theme';
  import type { SettingsBridge, SettingsSend } from './bridge';
  import { SECTIONS } from './sections';
  import type { AppSettingsView } from './view';

  let { bridge }: { bridge: SettingsBridge } = $props();

  let view = $state<AppSettingsView | null>(null);
  let stop: (() => void) | null = null;
  // The look comes from the view: Exile until a view says otherwise.
  const exile = $derived(themeOf(view?.theme) === 'exile');
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
        (error) => console.error('waystone-roller settings: the bridge did not start', error),
      );
    return () => {
      live = false;
    };
  });

  onDestroy(() => {
    stop?.();
  });
</script>

<!-- In Claude `.window-scroll` is a plain block with nothing of its own to
     draw, so the window is drawn as it was; switching the theme keeps every
     element. -->
<div
  class="window"
  data-state={view?.update.state ?? 'none'}
  data-install={view?.install.state ?? 'none'}
  style:--well-image={exile ? `url("${masonry}")` : null}
>
  {#if exile}<MaterialFrame />{/if}
  <main aria-labelledby="settings-title">
    <h1 class="title caps" id="settings-title">Settings</h1>
    <div class="window-scroll">
      {#if view}
        {#each SECTIONS as section (section.id)}
          <!-- The theme's own section is marked, so a comparison with the
               window before it can take the new section out and compare the rest. -->
          <section class="section" aria-labelledby="section-{section.id}" data-theme-setting={section.id === 'appearance' ? '' : undefined}>
            <h2 class="group-label" id="section-{section.id}">{section.title}</h2>
            <section.component {view} {send} />
          </section>
        {/each}
      {/if}
    </div>
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
