<script lang="ts">
  // The main window's root: renders the last `View` the bridge delivered and
  // turns a paste into the window into `grab_text`. The shell answers every
  // command with a new view; nothing here changes the view itself.
  import { onDestroy, onMount } from 'svelte';
  import type { Bridge, Command, CommandArgs } from './bridge';
  import type { View } from './ipc';
  import { pastedText } from './paste';
  import GrabPanel from './components/GrabPanel.svelte';

  let { bridge }: { bridge: Bridge } = $props();

  let view = $state<View | null>(null);
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
  <div class="app" data-state={view.grab ? 'grabbed' : 'empty'}>
    <header class="top">
      <h1 class="caps">Crafting Helper</h1>
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
    <GrabPanel {view} />
  </div>
{:else}
  <div class="app loading" aria-busy="true"></div>
{/if}

<style>
  /* The panel takes what the header leaves; its text block scrolls inside it. */
  .app {
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    height: 100%;
    min-width: 0;
  }

  .top {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0 calc(var(--space-unit) * 4);
    padding: calc(var(--space-unit) * 4) calc(var(--space-unit) * 5) calc(var(--space-unit) * 3);
    border-bottom: var(--border-width) solid var(--color-panel-divider);
  }

  h1 {
    margin: 0;
    font-size: 22px;
    line-height: 28px;
    color: var(--color-text-primary);
  }

  /* The header's last item, pushed to the right edge. */
  .settings {
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
