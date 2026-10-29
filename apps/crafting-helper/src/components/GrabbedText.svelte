<script lang="ts">
  // The "Grabbed text" disclosure: the line count, where the grab came from
  // and the raw text exactly as the app sent it, in a sunken monospace block
  // that scrolls and takes the focus, so the keyboard can scroll a long item.
  // Open in mode refused, closed under the item in planning and guiding.
  import type { GrabSource, GrabView } from '../ipc';

  let { grab, open = false }: { grab: GrabView; open?: boolean } = $props();

  const SOURCE_WORDS: Record<GrabSource, string> = {
    shortcut: 'from the hotkey',
    paste: 'from a paste',
  };

  const lines = $derived(`${grab.line_count} ${grab.line_count === 1 ? 'line' : 'lines'}`);
</script>

<details class="grabbed-text" {open}>
  <summary>Grabbed text</summary>
  <p class="grab-meta" data-source={grab.source}>
    <span class="count">{lines}</span>, <span class="source">{SOURCE_WORDS[grab.source]}</span>
  </p>
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <pre class="raw" tabindex="0" aria-label="Grabbed text">{grab.text}</pre>
</details>

<style>
  summary {
    color: var(--color-text-secondary);
    font-size: var(--size-step);
    line-height: var(--line-step);
    cursor: pointer;
  }

  summary:hover {
    color: var(--color-text-primary);
  }

  .grab-meta {
    margin: var(--space-unit) 0;
    color: var(--color-text-secondary);
    font-size: var(--size-step);
    line-height: var(--line-step);
  }

  .count {
    color: var(--color-text-primary);
    font-weight: 700;
    font-variant-numeric: tabular-nums;
  }

  /* The raw text as grabbed: monospace, kept as it is, scrolled inside its block. */
  .raw {
    max-height: 280px;
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
</style>
