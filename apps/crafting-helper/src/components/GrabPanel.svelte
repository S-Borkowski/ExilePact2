<script lang="ts">
  // The "last grabbed item" panel. With no grab it says how to grab: the
  // game's own Ctrl+Alt+C, then the app's hotkey or Ctrl+V here. With a grab
  // it shows the line count, where the grab came from and the raw text, as
  // the app sent it. The status line comes last, in both cases.
  import type { GrabSource, View } from '../ipc';

  let { view }: { view: View } = $props();

  const SOURCE_WORDS: Record<GrabSource, string> = {
    shortcut: 'from the hotkey',
    paste: 'from a paste',
  };

  const grab = $derived(view.grab);
  const lines = $derived(grab ? `${grab.line_count} ${grab.line_count === 1 ? 'line' : 'lines'}` : '');
</script>

<section class="panel" aria-labelledby="grab-title">
  <h2 class="group-label caps" id="grab-title">Last grabbed item</h2>

  {#if grab}
    <p class="meta" data-source={grab.source}>
      <span class="count">{lines}</span>
      <span class="source">{SOURCE_WORDS[grab.source]}</span>
    </p>
    <!-- Focusable, so the keyboard can scroll a long item (a scrollable
         region the keyboard cannot reach is an accessibility failure). -->
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <pre class="raw" tabindex="0" aria-label="Grabbed text">{grab.text}</pre>
  {:else}
    <div class="howto">
      <p>Nothing grabbed yet. To grab an item:</p>
      <ol>
        <li>In the game, hover the item and press <kbd>Ctrl+Alt+C</kbd>.</li>
        <li>Press <kbd class="hotkey">{view.hotkey}</kbd>, or press <kbd>Ctrl+V</kbd> in this window.</li>
      </ol>
    </div>
  {/if}

  <p class="status" role="status" aria-live="polite" data-kind={view.status?.kind ?? 'none'}>
    {#if view.status}
      {#if view.status.kind === 'error'}<span class="mark" aria-hidden="true">!</span>{/if}<span>{view.status.text}</span>
    {/if}
  </p>
</section>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    gap: var(--space-section-gap);
    min-height: 0;
    padding: calc(var(--space-unit) * 3) calc(var(--space-unit) * 5) calc(var(--space-unit) * 4);
  }

  .group-label {
    margin: 0;
    color: var(--color-text-muted);
    font-size: var(--size-act);
    line-height: var(--line-caption);
    letter-spacing: 0.03em;
  }

  .meta {
    display: flex;
    flex-wrap: wrap;
    gap: 0 calc(var(--space-unit) * 2);
    margin: 0;
    font-size: var(--size-step);
    line-height: var(--line-step);
  }

  .count {
    color: var(--color-text-primary);
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }

  .source {
    color: var(--color-text-secondary);
  }

  /* The raw text as grabbed: monospace, kept as it is, scrolled inside its block. */
  .raw {
    flex: 1;
    min-height: 120px;
    margin: 0;
    padding: calc(var(--space-unit) * 3);
    overflow: auto;
    border: var(--border-width) solid var(--color-panel-border);
    background: var(--color-panel-sunken);
    color: var(--color-text-primary);
    font-family: var(--mono);
    font-size: var(--size-step);
    line-height: var(--line-step);
    white-space: pre;
    user-select: text;
  }

  .howto {
    color: var(--color-text-secondary);
    font-size: var(--size-step);
    line-height: var(--line-step);
  }

  .howto p {
    margin: 0 0 var(--space-unit);
  }

  .howto ol {
    margin: 0;
    padding-left: calc(var(--space-unit) * 5);
  }

  .howto li + li {
    margin-top: var(--space-unit);
  }

  kbd {
    padding: 0 var(--space-unit);
    border: var(--border-width) solid var(--color-panel-border);
    background: var(--color-panel-raised);
    color: var(--color-text-primary);
    font-family: var(--mono);
    font-size: var(--size-caption);
    white-space: nowrap;
  }

  /* A row, so a wrapped error hangs its lines after the "!". */
  .status {
    display: flex;
    align-items: baseline;
    min-height: var(--line-step);
    margin: 0;
    color: var(--color-text-secondary);
    font-size: var(--size-step);
    line-height: var(--line-step);
  }

  .status[data-kind='error'] {
    color: var(--error);
  }

  /* An error starts with "!", so it is not told by its colour alone. */
  .mark {
    flex: none;
    margin-right: calc(var(--space-unit) * 1.5);
    font-weight: 700;
  }
</style>
