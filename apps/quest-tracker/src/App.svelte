<script lang="ts">
  // The overlay's root: renders the last `View` the bridge delivered. Which
  // frame, which mode, which rows and in what order are the view's to say;
  // this component only draws them. A live change of a quest's row
  // (`qt:row-change`) is played by that row (`QuestRow`) and announced here,
  // in a polite live region, whether the row is on screen or not.
  import { onDestroy, onMount } from 'svelte';
  import type { Bridge, ResizeDirection, Send } from './bridge';
  import type { RowChange, View } from './ipc';
  import { ANNOUNCED_KEEP, RowChanges, setRowChanges } from './rowchange';
  import Collapsed from './components/Collapsed.svelte';
  import Panel from './components/Panel.svelte';

  let { bridge }: { bridge: Bridge } = $props();

  let view = $state<View | null>(null);
  let announced = $state<Array<{ id: number; text: string }>>([]);
  const changes = setRowChanges(new RowChanges());
  let stop: (() => void) | null = null;

  const send: Send = (name, args) => {
    void bridge.command(name, args);
  };
  const drag = (): void => bridge.startDrag();
  const resize = (direction: ResizeDirection): void => bridge.startResize(direction);

  function onRowChange(change: RowChange): void {
    changes.add(change);
    // Each change is a new node: a polite region reads additions, so the
    // same sentence twice is still read twice.
    announced = [...announced, { id: change.id, text: change.text }].slice(-ANNOUNCED_KEEP);
  }

  onMount(() => {
    let live = true;
    bridge
      .start(
        (next) => {
          view = next;
        },
        onRowChange,
      )
      .then(
        (unsubscribe) => {
          if (live) stop = unsubscribe;
          else unsubscribe();
        },
        (error) => console.error('quest-tracker: the bridge did not start', error),
      );
    return () => {
      live = false;
    };
  });

  onDestroy(() => {
    stop?.();
  });
</script>

<div
  class="overlay"
  data-frame={view?.frame ?? 'none'}
  data-mode={view?.mode ?? 'play'}
  style:--panel-opacity={view?.settings.opacity ?? null}
>
  {#if view}
    {#if view.frame === 'collapsed'}
      <Collapsed {view} {send} />
    {:else}
      <Panel {view} {send} {drag} {resize} />
    {/if}
  {/if}
  <div class="announcer sr-only" aria-live="polite" aria-atomic="false">
    {#each announced as a (a.id)}
      <p>{a.text}</p>
    {/each}
  </div>
</div>

<style>
  /* The window's transparent margin holds the panel's shadow. The overlay is
     the window's height, so the panel fills the window inside that margin in
     both directions (the player sizes the window; the scale only zooms). The
     collapsed badge sits at the window's right edge: with no saved position
     the shell places the window where the old right dock did (the dock
     setting went after Live 2). The preview's stand-in for the webview zoom
     is a CSS `zoom` on the page, which scales `vh` with it, so the height
     is divided by that scale there; inside the app it is unset (1). */
  .overlay {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    box-sizing: border-box;
    height: calc(100vh / var(--preview-scale, 1));
    padding: 12px;
    overflow: hidden;
    color: var(--color-text-primary);
    font-family: var(--font-family);
    font-size: var(--size-step);
    font-weight: var(--font-weight-row);
    line-height: var(--line-step);
    font-kerning: normal;
    -webkit-font-smoothing: antialiased;
    user-select: none;
  }

  .overlay :global(input[type='text']) {
    user-select: text;
  }

  .announcer p {
    margin: 0;
  }
</style>
