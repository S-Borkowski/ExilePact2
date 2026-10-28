<script lang="ts">
  // The Updates section: the "Automatically check for updates" switch, what
  // the check last said and where an install stands (one polite live
  // region), the one action the install state offers (install, Cancel or
  // Try again), "Check now", and the running version. Each control sends one
  // command and shows what the next `wr:app-settings` says; the section keeps
  // no copy of its own. Nothing here opens a page: a failure is its reason
  // line and Try again.
  import type { SectionProps } from './sections';
  import { progressText } from './view';

  let { view, send }: SectionProps = $props();

  const update = $derived(view.update);
  const install = $derived(view.install);
  const checking = $derived(update.state === 'checking');
  /** An install runs: no check and no second install meanwhile (the app refuses both). */
  const installing = $derived(install.state === 'downloading' || install.state === 'verifying' || install.state === 'restarting');
  /** The install states that write their own status line, under the check's. */
  const installLine = $derived(install.state !== 'none' && install.state !== 'ready');
  const progress = $derived(install.state === 'downloading' ? install.progress : null);
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
  Asks GitHub once a day whether a newer version is out. Downloads nothing: an update is installed only when you press its button.
  Off: Waystone Roller never connects unless you ask it to.
</p>

<div class="check">
  <div class="status" role="status" aria-live="polite">
    <p class="check-line" data-state={update.state}>
      {#if update.state === 'failed'}<span class="mark" aria-hidden="true">!</span>{/if}<span>{update.text}</span>
    </p>
    {#if installLine}
      <p class="install-line" data-install={install.state}>
        {#if install.state === 'failed'}<span class="mark" aria-hidden="true">!</span>{/if}<span>{install.text}</span>
      </p>
    {/if}
  </div>

  {#if progress}
    <div class="progress">
      <!-- Determinate when the release lists the size; a bar with no value
           at all (indeterminate) when it does not. -->
      {#if progress.total === null}
        <progress class="bar" aria-label="Download"></progress>
      {:else}
        <progress class="bar" aria-label="Download" max={progress.total} value={progress.bytes}></progress>
      {/if}
      <span class="progress-text caption">{progressText(progress)}</span>
    </div>
  {/if}

  <div class="actions">
    {#if install.state === 'ready'}
      <button type="button" class="btn primary install" disabled={checking} onclick={() => send('install_update', {})}>{install.text}</button>
    {:else if install.state === 'downloading'}
      <button type="button" class="btn" onclick={() => send('cancel_update', {})}>Cancel</button>
    {:else if install.state === 'failed'}
      <button type="button" class="btn primary" disabled={checking} onclick={() => send('install_update', {})}>Try again</button>
    {/if}
    <button type="button" class="btn" disabled={checking || installing} onclick={() => send('check_for_updates', {})}>Check now</button>
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
    font-weight: 500;
    line-height: var(--line-caption);
  }

  /* The privacy promise is read, not skimmed: Label size, not Caption. */
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
    padding: var(--space-unit) 0 calc(var(--space-unit) * 3);
  }

  .status {
    display: flex;
    flex-direction: column;
    gap: var(--space-unit);
  }

  /* A row, so a wrapped failure hangs its lines after the "!" (review F1). */
  .status p {
    display: flex;
    align-items: baseline;
    margin: 0;
    font-size: var(--size-step);
    line-height: var(--line-step);
  }

  .check-line {
    color: var(--color-text-primary);
  }

  .check-line[data-state='available'] {
    color: var(--want);
  }

  .check-line[data-state='failed'],
  .install-line[data-install='failed'] {
    color: var(--error);
  }

  .install-line {
    color: var(--color-text-secondary);
  }

  /* A failure starts with "!", so it is not told by its colour alone. */
  .mark {
    flex: none;
    margin-right: calc(var(--space-unit) * 1.5);
    font-weight: 700;
  }

  .progress {
    display: flex;
    align-items: center;
    gap: var(--space-section-gap);
  }

  /* A flat, square bar on the sunken track; the accent fills what came. */
  .bar {
    flex: 1;
    min-width: 0;
    height: 6px;
    border: var(--border-width) solid var(--color-panel-border);
    background: var(--color-panel-sunken);
    appearance: none;
  }

  .bar::-webkit-progress-bar {
    background: var(--color-panel-sunken);
  }

  .bar::-webkit-progress-value {
    background: var(--color-edit-accent);
    transition: width 120ms linear;
  }

  .bar::-moz-progress-bar {
    background: var(--color-edit-accent);
  }

  /* No total known: a quarter of the track slides across; still under reduced motion. */
  .bar:indeterminate {
    background: linear-gradient(var(--color-edit-accent), var(--color-edit-accent)) no-repeat var(--color-panel-sunken);
    background-size: 25% 100%;
    animation: wr-slide 1.2s ease-in-out infinite alternate;
  }

  .bar:indeterminate::-webkit-progress-bar {
    background: transparent;
  }

  @keyframes wr-slide {
    from {
      background-position: 0% 0;
    }

    to {
      background-position: 100% 0;
    }
  }

  /* Lining, even-width figures: the count does not jitter as it climbs (review F3). */
  .progress-text {
    flex: none;
    font-variant-numeric: lining-nums tabular-nums;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-section-gap);
  }

  .version {
    font-variant-numeric: tabular-nums;
  }

  @media (prefers-reduced-motion: reduce) {
    .bar:indeterminate {
      animation: none;
      background: var(--color-edit-tint);
    }

    .bar::-webkit-progress-value {
      transition: none;
    }
  }
</style>
