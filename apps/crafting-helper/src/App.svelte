<script lang="ts">
  // The main window's root: renders the last `View` the bridge delivered and
  // turns a paste into the window into `grab_text`. The shell answers every
  // command with a new view; nothing here changes the view itself. The
  // catalogue the currency picker lists is read once, at the start.
  import { onDestroy, onMount } from 'svelte';
  import type { Bridge, Command, CommandArgs } from './bridge';
  import type { CatalogueGroup, View } from './ipc';
  import { pastedText } from './paste';
  import CraftArea from './craft/CraftArea.svelte';
  import StatusLine from './components/StatusLine.svelte';

  let { bridge }: { bridge: Bridge } = $props();

  const uid = $props.id();

  let view = $state<View | null>(null);
  let catalogue = $state<CatalogueGroup[]>([]);
  let stop: (() => void) | null = null;

  function send<C extends Command>(name: C, args: CommandArgs[C]): void {
    void bridge.command(name, args);
  }

  /** Ctrl+V anywhere in the window but an editable field: the pasted text is the grab. */
  function onPaste(event: Event): void {
    const text = pastedText(event);
    if (text === null) return;
    event.preventDefault();
    send('grab_text', { text });
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
        (error) => console.error('crafting-helper: the bridge did not start', error),
      );
    bridge.catalogue().then(
      (groups) => {
        if (live) catalogue = groups;
      },
      (error) => console.error('crafting-helper: the catalogue did not load', error),
    );
    return () => {
      live = false;
    };
  });

  onDestroy(() => {
    stop?.();
  });
</script>

<svelte:window onpaste={onPaste} />

{#if view}
  <div class="app" data-state={view.grab ? 'grabbed' : 'empty'} data-mode={view.craft.mode}>
    <header class="top">
      <h1 class="caps">Crafting Helper</h1>
      <!-- Sends the command alone: the app holds the one address it opens
           in the default browser, and this page holds none. -->
      <button type="button" class="btn coe" aria-describedby="{uid}-coe" onclick={() => send('open_craft_of_exile', {})}>
        <span>Explore more crafting on Craft of Exile</span>
        <svg class="out" viewBox="0 0 12 12" width="12" height="12" aria-hidden="true" focusable="false">
          <path d="M4.5 2.5h5v5M9.5 2.5 3 9" fill="none" stroke="currentColor" stroke-width="1.5" />
        </svg>
      </button>
      <span class="visually-hidden" id="{uid}-coe">Opens Craft of Exile in your web browser</span>
      <!-- Opens the Settings window; names a newer version a check found, in words. -->
      <button
        type="button"
        class="btn settings"
        aria-label={view.update_available ? `Settings, update ${view.update_available} available` : undefined}
        onclick={() => send('open_settings', {})}
      >
        <span>Settings</span>
        {#if view.update_available}
          <span class="update-mark"><span class="dot" aria-hidden="true"></span>Update {view.update_available}</span>
        {/if}
      </button>
    </header>
    <CraftArea {view} {catalogue} {send} />
    <StatusLine status={view.status} />
  </div>
{:else}
  <div class="app loading" aria-busy="true"></div>
{/if}

<style>
  /* The header, the craft area taking the rest, the status line last. */
  .app {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr) auto;
    height: 100%;
    min-width: 0;
  }

  /* From 760 px: title, then the two buttons at the right end. Below, the
     Craft of Exile button takes its own row under the title. */
  .top {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto auto;
    grid-template-areas: 'title coe settings';
    align-items: center;
    gap: calc(var(--space-unit) * 2) calc(var(--space-unit) * 3);
    padding: calc(var(--space-unit) * 4) calc(var(--space-unit) * 5) calc(var(--space-unit) * 3);
    border-bottom: var(--border-width) solid var(--color-panel-divider);
  }

  h1 {
    grid-area: title;
    margin: 0;
    font-size: 22px;
    line-height: 28px;
    color: var(--color-text-primary);
  }

  .coe {
    grid-area: coe;
  }

  .settings {
    grid-area: settings;
  }

  .out {
    flex: none;
  }

  @media (max-width: 759px) {
    .top {
      grid-template-columns: minmax(0, 1fr) auto;
      grid-template-areas:
        'title settings'
        'coe coe';
    }

    .coe {
      justify-self: start;
    }
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
