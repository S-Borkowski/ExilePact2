<script lang="ts">
  // The Updates section: the "Automatically check for updates" switch, what
  // the check last said, "Check now", the install of a newer version (its
  // button, the download's progress with *Cancel*, a failure's one-line
  // reason with *Try again*), and the running version. Each control sends
  // one command and shows what the next `qt:app-settings` says; the window
  // keeps no copy of its own. No state offers a release page: an update is
  // installed here or says why it cannot be.
  import Icon from '../components/Icon.svelte';
  import type { SectionProps } from './sections';
  import type { InstallState, UpdateState } from './view';

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

  /**
   * The same for the install's line; `none` and `ready` have no line (the
   * button says it). Every line has one, so its text lines up with the
   * check's above it; the refresh icon is "under way", as while checking.
   */
  const INSTALL_ICON: Record<InstallState, string | null> = {
    none: null,
    unsigned: 'ui-warning',
    ready: null,
    downloading: 'ui-refresh',
    verifying: 'ui-refresh',
    restarting: 'ui-check',
    failed: 'ui-warning',
  };

  /** While these run the app refuses a check (and a second install). */
  const INSTALL_BUSY: readonly InstallState[] = ['downloading', 'verifying', 'restarting'];

  const KIB = 1024;
  const MIB = 1024 * 1024;

  /** A byte count as a person reads it: whole KB under a megabyte, else MB to one place. */
  function amount(bytes: number): string {
    return bytes < MIB ? `${Math.round(bytes / KIB)} KB` : `${(bytes / MIB).toFixed(1)} MB`;
  }

  const update = $derived(view.update);
  const install = $derived(view.install);
  const icon = $derived(STATE_ICON[update.state]);
  const installIcon = $derived(INSTALL_ICON[install.state]);
  const checking = $derived(update.state === 'checking');
  const installing = $derived(INSTALL_BUSY.includes(install.state));
  const hasLine = $derived(install.state !== 'none' && install.state !== 'ready');
  const progress = $derived(install.state === 'downloading' ? install.progress : null);
  const determinate = $derived(progress !== null && progress.total !== null && progress.total > 0);
  const progressText = $derived(
    progress === null ? '' : progress.total === null ? amount(progress.bytes) : `${amount(progress.bytes)} of ${amount(progress.total)}`,
  );
  const fill = $derived(
    determinate && progress && progress.total ? `${Math.min(100, (progress.bytes / progress.total) * 100)}%` : '0%',
  );
</script>

<label class="row">
  <span class="label">Automatically check for updates</span>
  <input
    type="checkbox"
    class="toggle"
    role="switch"
    checked={view.check_updates}
    aria-describedby="updates-help"
    onchange={(e) => send('update_settings', { patch: { check_updates: e.currentTarget.checked } })}
  />
</label>
<p class="help" id="updates-help">
  Asks GitHub once a day whether a newer version is out. Downloads nothing. Off: Quest Tracker never connects unless you ask it to
  check.
</p>

<div class="check">
  <div class="lines">
    <p class="status" data-state={update.state} role="status" aria-live="polite">
      {#if icon}<span class="status-icon"><Icon id={icon} size="16px" /></span>{/if}
      <span>{update.text}</span>
    </p>
    <!-- The install's own line: always in the page, empty while the install has
         nothing to say, so a screen reader is already listening when it does. -->
    <div class="install" class:filled={hasLine} data-install={install.state}>
      <p class="status" id="install-line" data-install={install.state} role="status" aria-live="polite">
        {#if hasLine}
          {#if installIcon}<span class="status-icon"><Icon id={installIcon} size="16px" /></span>{/if}
          <span>{install.text}</span>
        {/if}
      </p>
      {#if progress}
        <div class="progress">
          {#if determinate}
            <div
              class="bar"
              role="progressbar"
              aria-labelledby="install-line"
              aria-valuemin="0"
              aria-valuemax={progress.total}
              aria-valuenow={progress.bytes}
              aria-valuetext={progressText}
              style:--fill={fill}
            ></div>
          {:else}
            <div class="bar open" role="progressbar" aria-labelledby="install-line" aria-valuetext={progressText}></div>
          {/if}
          <span class="caption amount">{progressText}</span>
        </div>
      {/if}
    </div>
  </div>
  <div class="actions">
    <button type="button" class="btn cut" disabled={checking || installing} onclick={() => send('check_for_updates', {})}>
      Check now
    </button>
    {#if install.state === 'ready'}
      <button type="button" class="btn cut primary" disabled={checking} onclick={() => send('install_update', {})}>
        {install.text}
      </button>
    {:else if install.state === 'downloading'}
      <button type="button" class="btn cut" onclick={() => send('cancel_update', {})}>Cancel</button>
    {:else if install.state === 'failed'}
      <button type="button" class="btn cut" disabled={checking} onclick={() => send('install_update', {})}>Try again</button>
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

  /* The install's part takes no room, and adds no gap, while it is empty. */
  .install.filled {
    display: flex;
    flex-direction: column;
    gap: var(--space-section-gap);
    margin-top: var(--space-section-gap);
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

  .status[data-state='failed'] .status-icon,
  .status[data-install='failed'] .status-icon,
  .status[data-install='unsigned'] .status-icon {
    color: var(--color-warning);
  }

  /* The download: the slider's 4 px track, filled by the accent up to the
     bytes received, and the amount in words beside it; it starts where the
     line's text does, past the 16 px icon. */
  .progress {
    display: flex;
    align-items: center;
    gap: var(--space-section-gap);
    padding-left: calc(16px + var(--space-section-gap));
  }

  .bar {
    flex: 1;
    min-width: 0;
    height: 4px;
    background: linear-gradient(
      to right,
      var(--color-edit-accent) var(--fill),
      var(--color-panel-divider) var(--fill)
    );
  }

  /* No total known: the track alone, dimly filled, and no motion. */
  .bar.open {
    background: var(--color-panel-divider);
    opacity: 0.8;
  }

  .amount {
    flex: none;
    font-variant-numeric: tabular-nums;
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
