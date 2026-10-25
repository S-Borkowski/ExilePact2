<script lang="ts">
  // The overlay's root: renders the last `View` the bridge delivered and the
  // toast stack below it. Which frame, which mode, which rows and in what
  // order are the view's to say; this component only draws them.
  import { onDestroy, onMount } from 'svelte';
  import type { Bridge, Send } from './bridge';
  import type { QuestAction, Toast, View } from './ipc';
  import { type ShownToast, TOAST_MS, UNDO_MS, dropToast, fromToast, pushToast } from './toasts';
  import Collapsed from './components/Collapsed.svelte';
  import Panel from './components/Panel.svelte';
  import Toasts from './components/Toasts.svelte';

  let { bridge }: { bridge: Bridge } = $props();

  let view = $state<View | null>(null);
  let toasts = $state<ShownToast[]>([]);
  const timers = new Set<ReturnType<typeof setTimeout>>();
  let stop: (() => void) | null = null;
  let undoSeq = 0;

  const send: Send = (name, args) => {
    void bridge.command(name, args);
  };

  function show(toast: ShownToast, ms: number): void {
    toasts = pushToast(toasts, toast);
    if (bridge.holdToasts) return;
    const timer = setTimeout(() => {
      timers.delete(timer);
      toasts = dropToast(toasts, toast.key);
    }, ms);
    timers.add(timer);
  }

  function onToast(toast: Toast): void {
    show(fromToast(toast), TOAST_MS);
  }

  // Mark done and Skip in edit mode offer an Undo for 5 s (the brief).
  const UNDO: Partial<Record<QuestAction, { action: QuestAction; text: string; icon: string }>> = {
    mark_done: { action: 'unmark_done', text: 'Marked done', icon: 'status-done' },
    skip: { action: 'unskip', text: 'Skipped', icon: 'status-skipped' },
  };

  function onAction(questId: string, action: QuestAction): void {
    const undo = UNDO[action];
    if (!undo) return;
    undoSeq += 1;
    show({ key: `undo-${undoSeq}`, kind: 'undo', text: undo.text, icon: undo.icon, undo: { questId, action: undo.action } }, UNDO_MS);
  }

  function onUndo(toast: ShownToast): void {
    if (toast.undo) send('quest_action', toast.undo);
    toasts = dropToast(toasts, toast.key);
  }

  onMount(() => {
    let live = true;
    bridge
      .start(
        (next) => {
          view = next;
        },
        onToast,
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
    for (const timer of timers) clearTimeout(timer);
    timers.clear();
  });
</script>

<div
  class="overlay"
  data-frame={view?.frame ?? 'none'}
  data-mode={view?.mode ?? 'play'}
  data-dock={view?.settings.dock ?? 'right'}
  style:--panel-opacity={view?.settings.opacity ?? null}
>
  {#if view}
    {#if view.frame === 'collapsed'}
      <Collapsed {view} {send} />
    {:else}
      <Panel {view} {send} {onAction} />
    {/if}
  {/if}
  <Toasts {toasts} edit={view?.mode === 'edit'} {onUndo} />
</div>

<style>
  .overlay {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    padding: 12px;
    font-family: var(--font-family);
    color: var(--color-text-primary);
  }

  .overlay[data-dock='left'] {
    align-items: flex-start;
  }
</style>
