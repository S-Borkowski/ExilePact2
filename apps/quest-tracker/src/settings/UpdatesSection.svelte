<script lang="ts">
  // The Updates section: the "Automatically check for updates" switch, what
  // the check last said, "Check now", "Open the release page" while a newer
  // version is known, and the running version. Each control sends one
  // command and shows what the next `qt:app-settings` says; the window keeps
  // no copy of its own.
  import Icon from '../components/Icon.svelte';
  import type { SectionProps } from './sections';
  import type { UpdateState } from './view';

  let { view, send }: SectionProps = $props();

  /** The icon that repeats each state's text, and its colour; none where the text alone fits. */
  const STATE_ICON: Record<UpdateState, string | null> = {
    idle: null,
    checking: 'ui-refresh',
    'up-to-date': 'ui-check',
    available: 'status-available',
    'no-release': null,
    failed: 'ui-warning',
  };

  const update = $derived(view.update);
  const icon = $derived(STATE_ICON[update.state]);
  const checking = $derived(update.state === 'checking');
</script>

<label class="row">
  <span class="label">Automatically check for updates</span>
  <input
    type="checkbox"
    class="toggle"
    role="switch"
    checked={view.check_updates}
    aria-describedby="updates-help"
    onchange={() => send('update_settings', { patch: { check_updates: !view.check_updates } })}
  />
</label>
<p class="help" id="updates-help">
  Asks GitHub once a day whether a newer version is out. Downloads nothing. Off: Quest Tracker never connects unless you ask it to
  check.
</p>

<div class="check">
  <p class="status" data-state={update.state} role="status" aria-live="polite">
    {#if icon}<span class="status-icon"><Icon id={icon} size="16px" /></span>{/if}
    <span>{update.text}</span>
  </p>
  <div class="actions">
    <button type="button" class="btn cut" disabled={checking} onclick={() => send('check_for_updates', {})}>Check now</button>
    {#if update.newer !== null}
      <button type="button" class="btn cut primary" onclick={() => send('open_release_page', {})}>
        Open the release page <Icon id="ui-external-link" size="14px" />
      </button>
    {/if}
  </div>
  <p class="caption version">Version {view.version}</p>
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-row-pad);
    min-height: 40px;
    cursor: pointer;
  }

  .label {
    color: var(--color-text-primary);
    font-size: var(--size-label);
    font-weight: var(--font-weight-row);
    line-height: var(--line-caption);
  }

  /* The privacy promise is read, not skimmed: Label size, not the panel's
     12 px Caption (design review, F2). */
  .help {
    margin: -6px 0 0;
    padding-bottom: var(--space-section-gap);
    color: var(--color-text-muted);
    font-size: var(--size-label);
    line-height: var(--line-step);
    letter-spacing: 0.2px;
  }

  .check {
    display: flex;
    flex-direction: column;
    gap: var(--space-section-gap);
    padding: var(--space-unit) 0 var(--space-section-gap);
  }

  .status {
    display: flex;
    align-items: flex-start;
    gap: var(--space-section-gap);
    margin: 0;
    color: var(--color-text-primary);
    font-size: var(--size-step);
    line-height: var(--line-step);
  }

  /* The icon sits on the first line's middle, however the text wraps. */
  .status-icon {
    display: inline-flex;
    align-items: center;
    height: var(--line-step);
    color: var(--color-text-secondary);
  }

  .status[data-state='available'] .status-icon {
    color: var(--color-status-available);
  }

  .status[data-state='failed'] .status-icon {
    color: var(--color-warning);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-section-gap);
  }

  .version {
    font-variant-numeric: tabular-nums;
  }
</style>
