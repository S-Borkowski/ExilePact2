<script lang="ts">
  // The page's root: renders the last `View` the bridge delivered and hands
  // each control a `send`. The shell answers every command with a new view;
  // nothing here changes the view itself.
  import { onDestroy, onMount } from 'svelte';
  import type { Bridge, Command, CommandArgs } from './bridge';
  import type { View } from './ipc';
  import { sourceSite } from './selection';
  import FilterRow from './components/FilterRow.svelte';
  import ModList from './components/ModList.svelte';
  import SearchFooter from './components/SearchFooter.svelte';

  let { bridge }: { bridge: Bridge } = $props();

  let view = $state<View | null>(null);
  let stop: (() => void) | null = null;

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
  <div class="app" data-state={view.result.state}>
    <header class="top">
      <h1 class="caps">Waystone Roller</h1>
      <p class="source">Mods from {sourceSite(view)}, fetched {view.data.fetched}</p>
    </header>
    <FilterRow selection={view.selection} {send} />
    <ModList {view} {send} />
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
</style>
