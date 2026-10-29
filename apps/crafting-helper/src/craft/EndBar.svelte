<script lang="ts">
  // The End bar (`ui-design.md` § "Mode guiding"): End craft sends
  // `end_craft`. In done and replan it is the primary button and acts at
  // once; in active and stuck, where it would throw a craft in progress
  // away, it first asks in place, without a dialog. The question takes the
  // focus on its Keep crafting button, so a second Enter does not end the
  // craft, and Keep crafting gives the focus back. The caller keys this bar
  // by the guide's status, so a new status closes the question.
  import { tick } from 'svelte';
  import type { Send } from '../bridge';
  import type { GuideStatus } from '../ipc';

  let { status, send }: { status: GuideStatus; send: Send } = $props();

  const uid = $props.id();
  const atOnce = $derived(status === 'done' || status === 'replan');

  let asking = $state(false);
  let opener = $state<HTMLButtonElement | null>(null);
  let keep = $state<HTMLButtonElement | null>(null);

  async function ask(): Promise<void> {
    asking = true;
    await tick();
    keep?.focus();
  }

  async function keepCrafting(): Promise<void> {
    asking = false;
    await tick();
    opener?.focus();
  }

  function end(): void {
    send('end_craft', {});
  }
</script>

<div class="end-bar">
  {#if atOnce}
    <button type="button" class="btn primary" onclick={end}>End craft</button>
  {:else if asking}
    <div class="question" role="group" aria-labelledby="{uid}-question">
      <p id="{uid}-question">End this craft? Its history is cleared.</p>
      <div class="actions">
        <button type="button" class="btn" onclick={end}>End craft</button>
        <button type="button" class="btn" bind:this={keep} onclick={keepCrafting}>Keep crafting</button>
      </div>
    </div>
  {:else}
    <button type="button" class="btn" bind:this={opener} onclick={ask}>End craft</button>
  {/if}
</div>

<style>
  .end-bar {
    display: flex;
    align-items: center;
    gap: calc(var(--space-unit) * 3);
    padding: calc(var(--space-unit) * 3) calc(var(--space-unit) * 5);
    border-top: var(--border-width) solid var(--color-panel-divider);
    background: var(--color-panel-bg);
  }

  .question {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: calc(var(--space-unit) * 2) calc(var(--space-unit) * 3);
  }

  .question p {
    margin: 0;
    color: var(--color-text-primary);
    font-size: var(--size-step);
    line-height: var(--line-step);
  }

  .actions {
    display: flex;
    gap: calc(var(--space-unit) * 2);
  }
</style>
