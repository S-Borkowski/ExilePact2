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
  import { applyPresentationTheme } from './theme';

  let { bridge }: { bridge: Bridge } = $props();

  let view = $state<View | null>(null);
  let announced = $state<Array<{ id: number; text: string }>>([]);
  const changes = setRowChanges(new RowChanges());
  let stop: (() => void) | null = null;
  $effect(() => { applyPresentationTheme(document.documentElement, view?.settings.theme ?? 'exile'); });

  const send: Send = (name, args) => {
    void bridge.command(name, args);
  };
  const drag = (): void => bridge.startDrag();
  const resize = (direction: ResizeDirection): void => bridge.startResize(direction);

  // The webview zoom the scale setting applies, as the page sees it: the
  // window keeps its size (`outerWidth`) while the viewport shrinks to the
  // window over the zoom (`innerWidth`), and a zoom change fires `resize`.
  // The overlay divides its margin by it. The preview has no webview zoom:
  // its CSS stand-in sets `--page-zoom` on the page itself.
  let pageZoom = $state<number | null>(null);
  function measureZoom(): void {
    const zoom = window.outerWidth / window.innerWidth;
    pageZoom = Number.isFinite(zoom) && zoom > 0 ? zoom : 1;
  }

  function onRowChange(change: RowChange): void {
    changes.add(change);
    // Each change is a new node: a polite region reads additions, so the
    // same sentence twice is still read twice.
    announced = [...announced, { id: change.id, text: change.text }].slice(-ANNOUNCED_KEEP);
  }

  onMount(() => {
    let live = true;
    const inApp = bridge.kind === 'tauri';
    if (inApp) {
      measureZoom();
      window.addEventListener('resize', measureZoom);
    }
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
      if (inApp) window.removeEventListener('resize', measureZoom);
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
  data-theme={view?.settings.theme ?? 'exile'}
  style:--panel-opacity={view?.settings.opacity ?? null}
  style:--page-zoom={pageZoom}
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
     setting went after Live 2).
     Inside the app the webview zoom shrinks the viewport to the window over
     the zoom, so `100vh` is the window's height at every scale, but draws
     every CSS px that many times larger: the margin is divided by the zoom
     (`--page-zoom`) to stay 12 window pixels, and the panel's frame keeps
     its size while its content zooms. The preview's stand-in for the
     webview zoom is a CSS `zoom` on the page, which leaves the viewport
     alone and scales `vh` with it, so the height is divided by that scale
     there (`--preview-scale`, unset in the app) and the margin by the same
     scale, which the preview sets as `--page-zoom`. */
  .overlay {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    box-sizing: border-box;
    height: calc(100vh / var(--preview-scale, 1));
    padding: calc(12px / var(--page-zoom, 1));
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
