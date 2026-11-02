<script lang="ts">
  // The page's root: renders the last `View` the bridge delivered and hands
  // each control a `send`. The shell answers every command with a new view;
  // nothing here changes the view itself.
  import { onDestroy, onMount } from 'svelte';
  import type { Bridge, Command, CommandArgs } from './bridge';
  import type { View } from './ipc';
  import { sourceSite } from './selection';
  import { applyTheme, themeOf } from './theme';
  import FilterRow from './components/FilterRow.svelte';
  import MaterialFrame from './components/MaterialFrame.svelte';
  import ModList from './components/ModList.svelte';
  import SearchFooter from './components/SearchFooter.svelte';
  import masonry from '../theme/materials/raster/masonry-charcoal.png';

  let { bridge }: { bridge: Bridge } = $props();

  let view = $state<View | null>(null);
  let stop: (() => void) | null = null;
  // The look comes from the view: Exile until a view says otherwise.
  const exile = $derived(themeOf(view?.theme) === 'exile');
  $effect(() => {
    applyTheme(document.documentElement, view?.theme);
  });

  function send<C extends Command>(name: C, args: CommandArgs[C]): void {
    void bridge.command(name, args);
  }

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
        (error) => console.error('waystone-roller: the bridge did not start', error),
      );
    return () => {
      live = false;
    };
  });

  onDestroy(() => {
    stop?.();
  });
</script>

{#if view}
  <!--
    Exile draws the steel/masonry kit's frame over the window and a masonry
    well behind the page (`steel.css`); the page then scrolls in
    `.app-scroll`, inside the frame's clear interior, with the footer pinned
    below it, so no line or control ever crosses the art. In Claude
    `.app-scroll` has no box of its own and the page is drawn as it was.
    Switching the theme keeps every element (the list's filter text stays).
  -->
  <div class="app" data-state={view.result.state} style:--well-image={exile ? `url("${masonry}")` : null}>
    {#if exile}<MaterialFrame />{/if}
    <div class="app-scroll">
      <header class="top">
        <h1 class="caps">Waystone Roller</h1>
        <p class="source">Mods from {sourceSite(view)}, fetched {view.data.fetched}</p>
        <!-- Opens the Settings window; names a newer version a check found, in words. -->
        <button
          type="button"
          class="btn settings"
          aria-label={view.update_available ? `Settings, update ${view.update_available} available` : undefined}
          onclick={() => send('open_settings', {})}
        >
          <span>Settings</span>
          {#if view.update_available}
            <span class="update-mark"><span class="dot" aria-hidden="true" data-art-placeholder="wr-update-dot"></span>Update {view.update_available}</span>
          {/if}
        </button>
      </header>
      <FilterRow selection={view.selection} {send} />
      <ModList {view} {send} />
    </div>
    <SearchFooter {view} {send} />
  </div>
{:else}
  <div class="app loading" aria-busy="true"></div>
{/if}

<style>
  /*
   * The list takes what the other parts leave, but never less than a few
   * rows: on a short window the page scrolls instead, and the footer stays
   * pinned to the bottom (it is sticky in its own component).
   */
  .app {
    display: grid;
    grid-template-rows: auto auto minmax(200px, 1fr) auto;
    height: 100%;
    min-width: 0;
    overflow-y: auto;
  }

  /* Exile's scroller (`steel.css`); in Claude its children are the rows above. */
  .app-scroll {
    display: contents;
  }

  .top {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 0 calc(var(--space-unit) * 4);
    padding: calc(var(--space-unit) * 4) calc(var(--space-unit) * 5) calc(var(--space-unit) * 3);
  }

  h1 {
    margin: 0;
    font-size: 22px;
    line-height: 28px;
    color: var(--color-text-primary);
  }

  .source {
    margin: 0;
    color: var(--color-text-muted);
    font-size: var(--size-caption);
    line-height: var(--line-caption);
  }

  /* The header's last item, pushed to the right edge. */
  .settings {
    align-self: center;
    margin-left: auto;
  }

  /* An available update: its version in words, a green dot beside it. */
  .update-mark {
    display: inline-flex;
    align-items: center;
    gap: calc(var(--space-unit) * 1.5);
    padding-left: calc(var(--space-unit) * 2);
    border-left: var(--border-width) solid var(--color-panel-divider);
    color: var(--want);
    font-weight: 700;
  }

  .dot {
    width: 6px;
    height: 6px;
    background: var(--want);
  }
</style>
