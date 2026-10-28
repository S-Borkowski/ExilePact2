<script lang="ts">
  // The footer, always in view: the search string, the meter, each part's
  // length when the search is too long, Copy, the hint, the shell's status,
  // the paste checkbox and the shortcut field. Copy and the shortcut run one
  // copy path in the shell; only the shortcut also pastes into the game, and
  // only with the checkbox on. The page offers Copy only when the shell has a
  // search to copy.
  import type { Send } from '../bridge';
  import type { PartKind, ResultState, View } from '../ipc';

  let { view, send }: { view: View; send: Send } = $props();

  const result = $derived(view.result);
  const over = $derived(result.state === 'too_long');
  const longest = $derived(
    result.parts.length === 0 ? -1 : result.parts.reduce((best, p, i, all) => (p.length > all[best].length ? i : best), 0),
  );
  const unmatchableLabels = $derived(
    result.unmatchable.map((id) => view.mods.find((m) => m.id === id)?.label ?? id),
  );

  const PLACEHOLDERS: Record<Exclude<ResultState, 'ok'>, string> = {
    empty: 'Nothing selected yet',
    too_long: 'Too long to copy: see the parts below',
    unmatchable: 'No search: some wanted mods cannot be matched',
  };
  const NOT_COPYABLE: Record<Exclude<ResultState, 'ok'>, string> = {
    empty: 'Nothing to copy',
    too_long: 'Over the game’s limit',
    unmatchable: 'Cannot be matched',
  };
  const PART_NAMES: Record<PartKind, string> = {
    rarity: 'Rarity',
    tier: 'Tier',
    want: 'Wanted',
    avoid: 'Avoided',
    corruption: 'Corruption',
  };

  // The shortcut field holds what the player types until the shell's view
  // names another shortcut; then it shows that one.
  let draft = $state('');
  let shownHotkey: string | null = null;
  $effect.pre(() => {
    if (view.hotkey !== shownHotkey) {
      shownHotkey = view.hotkey;
      draft = view.hotkey;
    }
  });

  function setHotkey(event: SubmitEvent): void {
    event.preventDefault();
    const hotkey = draft.trim();
    if (hotkey !== '') send('set_hotkey', { hotkey });
  }

  // The checkbox shows the shell's `auto_paste` until the next view answers,
  // so a refused change never stays ticked.
  function setAutoPaste(event: Event & { currentTarget: HTMLInputElement }): void {
    const on = event.currentTarget.checked;
    event.currentTarget.checked = view.auto_paste;
    send('set_auto_paste', { on });
  }
</script>

<footer class="footer" aria-label="Search">
  <div class="string-row">
    <div class="string-box" data-state={result.state}>
      {#if result.state === 'ok'}
        <code class="search-string">{result.regex}</code>
      {:else}
        <span class="search-string placeholder">{PLACEHOLDERS[result.state]}</span>
      {/if}
    </div>
    <div class="copy-col">
      <button type="button" class="btn primary copy" disabled={result.state !== 'ok'} onclick={() => send('copy_now', {})}>
        Copy
      </button>
      {#if result.state !== 'ok'}
        <span class="copy-reason">{NOT_COPYABLE[result.state]}</span>
      {/if}
    </div>
  </div>

  <div class="meta-row">
    <p class="meter" data-over={over}>
      <span class="count">{result.length} / {result.limit}</span>
      {#if over}
        <span class="over">over by {result.length - result.limit}</span>
      {/if}
    </p>
    <p class="hint">
      <kbd>{view.hotkey}</kbd>
      {#if view.auto_paste}copies this and pastes it into the game{:else}copies this while you play{/if}
    </p>
  </div>

  {#if over && result.parts.length > 0}
    <div class="parts-box">
      <p class="parts-caption">Characters each rule takes, in the search’s order:</p>
      <ol class="parts">
        {#each result.parts as part, i (i)}
          <li class="part" data-what={part.what}>
            <span class="part-what">{PART_NAMES[part.what]}</span>
            <code class="part-text" title={part.text}>{part.text}</code>
            <span class="part-length">{part.length}</span>
            {#if i === longest}
              <span class="longest">longest</span>
            {/if}
          </li>
        {/each}
      </ol>
    </div>
  {/if}

  {#if result.state === 'unmatchable'}
    <div class="unmatchable-notice">
      <p>Some wanted mods cannot be told apart from others:</p>
      <ul>
        {#each unmatchableLabels as label, i (i)}
          <li>{label}</li>
        {/each}
      </ul>
    </div>
  {/if}

  <div class="bottom-row">
    <p class="status" role="status" data-kind={view.status?.kind ?? 'none'}>
      {#if view.status}
        {#if view.status.kind === 'error'}<span class="bang" aria-hidden="true">!</span>{/if}
        {view.status.text}
      {/if}
    </p>
    <div class="shortcut-controls">
      <label class="auto-paste-label">
        <input class="auto-paste" type="checkbox" checked={view.auto_paste} onchange={setAutoPaste} />
        <span>Paste into the game's search box</span>
      </label>
      <form class="shortcut" onsubmit={setHotkey}>
        <label class="shortcut-label">
          <span class="caps">Shortcut</span>
          <input class="field" type="text" bind:value={draft} placeholder="e.g. Ctrl+Alt+W" aria-describedby="shortcut-help" autocomplete="off" spellcheck="false" />
        </label>
        <button type="submit" class="btn">Set</button>
        <span class="visually-hidden" id="shortcut-help">For example Ctrl+Alt+W. The old shortcut stays if the new one is taken.</span>
      </form>
    </div>
  </div>
</footer>

<style>
  .footer {
    position: sticky;
    bottom: 0;
    z-index: 2;
    display: flex;
    flex-direction: column;
    gap: calc(var(--space-unit) * 2.5);
    padding: calc(var(--space-unit) * 3) calc(var(--space-unit) * 5);
    background: var(--color-panel-raised);
    border-top: var(--border-width) solid var(--color-panel-border);
    box-shadow: 0 -6px 16px rgba(0, 0, 0, 0.35);
  }

  .string-row {
    display: flex;
    align-items: flex-start;
    gap: calc(var(--space-unit) * 3);
  }

  .string-box {
    flex: 1 1 auto;
    min-width: 0;
    min-height: 44px;
    max-height: 96px;
    overflow-y: auto;
    padding: calc(var(--space-unit) * 2.5) calc(var(--space-unit) * 3);
    background: var(--color-panel-sunken);
    border: var(--border-width) solid var(--color-panel-divider);
  }

  .string-box[data-state='ok'] {
    border-color: var(--color-panel-border);
  }

  code.search-string {
    font-family: var(--mono);
    font-size: 14px;
    line-height: 20px;
    color: var(--color-text-primary);
    word-break: break-all;
    user-select: all;
  }

  .placeholder {
    color: var(--color-text-muted);
    font-size: var(--size-step);
  }

  .copy-col {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: var(--space-unit);
    flex: none;
    width: 112px;
  }

  .copy {
    min-height: 44px;
    font-size: var(--size-row);
  }

  .copy-reason {
    color: var(--color-text-muted);
    font-size: var(--size-caption);
    line-height: var(--line-caption);
    text-align: center;
  }

  .meta-row,
  .bottom-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: calc(var(--space-unit) * 2) calc(var(--space-unit) * 4);
  }

  .meter,
  .hint,
  .status,
  .parts-caption,
  .unmatchable-notice p {
    margin: 0;
  }

  .meter {
    display: flex;
    gap: calc(var(--space-unit) * 2);
    font-variant-numeric: tabular-nums;
    color: var(--color-text-secondary);
    font-size: var(--size-step);
  }

  .meter[data-over='true'] {
    color: var(--error);
    font-weight: 700;
  }

  .hint {
    color: var(--color-text-muted);
    font-size: var(--size-step);
  }

  kbd {
    padding: 0 var(--space-unit);
    border: var(--border-width) solid var(--color-panel-border);
    background: var(--color-panel-sunken);
    color: var(--color-text-secondary);
    font-family: var(--font-family);
    font-size: var(--size-caption);
  }

  .parts-box {
    padding: calc(var(--space-unit) * 2) calc(var(--space-unit) * 3);
    border: var(--border-width) solid color-mix(in srgb, var(--error) 50%, var(--color-panel-divider));
    background: var(--color-panel-sunken);
  }

  .parts-caption {
    color: var(--color-text-secondary);
    font-size: var(--size-caption);
    line-height: var(--line-caption);
  }

  .parts {
    margin: var(--space-unit) 0 0;
    padding: 0;
    list-style: none;
    max-height: 132px;
    overflow-y: auto;
  }

  .part {
    display: grid;
    grid-template-columns: 80px minmax(0, 1fr) 40px 56px;
    align-items: baseline;
    gap: calc(var(--space-unit) * 3);
    padding: 2px 0;
    font-size: var(--size-step);
  }

  .part-what {
    color: var(--color-text-secondary);
  }

  .part-text {
    overflow: hidden;
    font-family: var(--mono);
    font-size: 12px;
    color: var(--color-text-primary);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .part-length {
    text-align: right;
    font-variant-numeric: tabular-nums;
    color: var(--color-text-primary);
  }

  .longest {
    color: var(--error);
    font-size: var(--size-caption);
    font-weight: 700;
  }

  .unmatchable-notice {
    color: var(--error);
    font-size: var(--size-step);
  }

  .unmatchable-notice ul {
    margin: var(--space-unit) 0 0;
    padding-left: calc(var(--space-unit) * 5);
    color: var(--color-text-primary);
  }

  .status {
    flex: 1 1 280px;
    min-height: var(--line-row);
    color: var(--color-text-secondary);
    font-size: var(--size-step);
  }

  .status[data-kind='error'] {
    color: var(--error);
  }

  .bang {
    margin-right: 0.35em;
    font-weight: 800;
  }

  .shortcut-controls {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: calc(var(--space-unit) * 2) calc(var(--space-unit) * 4);
  }

  .auto-paste-label {
    display: inline-flex;
    align-items: center;
    gap: calc(var(--space-unit) * 1.5);
    min-height: var(--size-button);
    color: var(--color-text-secondary);
    font-size: var(--size-label);
    cursor: pointer;
  }

  .auto-paste {
    width: 16px;
    height: 16px;
    margin: 0;
    accent-color: var(--color-text-secondary);
  }

  .shortcut {
    display: flex;
    align-items: center;
    gap: calc(var(--space-unit) * 2);
  }

  .shortcut-label {
    display: flex;
    align-items: center;
    gap: calc(var(--space-unit) * 2);
    color: var(--color-text-secondary);
    font-size: var(--size-label);
  }

  .shortcut input {
    width: 150px;
  }
</style>
