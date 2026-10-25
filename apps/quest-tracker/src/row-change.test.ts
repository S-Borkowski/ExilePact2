import { flushSync } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import questRowSource from './components/QuestRow.svelte?raw';
import type { View } from './ipc';
import { NEW_CHARACTER } from './labels';
import { ANNOUNCED_KEEP, ROW_STEP_IN_MS, ROW_STEP_OUT_MS } from './rowchange';
import { type Rendered, fakeBridge, fixtureView, renderApp, rowChangeFor } from './test-support';

// The row's own transition, in place of the toasts (owner, 2026-10-25, after
// Live 1): `qt:row-change` plays on the row whose id is the change's
// `quest_id` and is announced in a polite live region.

let rendered: Rendered | null = null;

afterEach(() => {
  rendered?.destroy();
  rendered = null;
  vi.useRealTimers();
  delete (window as { matchMedia?: unknown }).matchMedia;
});

async function render(view: View) {
  const bridge = fakeBridge(view);
  rendered = await renderApp(bridge);
  // Rendered on real timers (renderApp waits on them); the transition runs on fake ones.
  vi.useFakeTimers();
  return { root: rendered.root, bridge };
}

/** Windows' animation setting as the webview reports it. */
function setReducedMotion(reduce: boolean): void {
  Object.defineProperty(window, 'matchMedia', {
    configurable: true,
    value: (query: string) => ({ matches: reduce && query.includes('prefers-reduced-motion: reduce'), media: query }),
  });
}

/** Moves the fake clock, then lets Svelte draw what the timers changed. */
function advance(ms: number): void {
  vi.advanceTimersByTime(ms);
  flushSync();
}

const rowOf = (root: HTMLElement, id: string) => root.querySelector<HTMLElement>(`[data-quest="${id}"]`)!;
const steps = (row: HTMLElement) => [...row.querySelectorAll('.step')];
const announcer = (root: HTMLElement) => root.querySelector('[aria-live="polite"]')!;
const announced = (root: HTMLElement) => [...announcer(root).querySelectorAll('p')].map((p) => p.textContent);

describe('the row transition', () => {
  it('row_step_transition_on_change', async () => {
    const view = fixtureView('panel-next-up', 'play');
    const king = view.next_up.find((r) => r.id === 'fixture-king')!;
    expect(king.current_step).toBe('Face the king');
    const { root, bridge } = await render(view);
    expect(root.querySelectorAll('[data-change]')).toHaveLength(0);

    // The shell sends the view first, then the change: the row's step is
    // already the new one, and the completed step plays out over it.
    bridge.pushView(fixtureView('panel-next-up', 'play'));
    bridge.pushRowChange(rowChangeFor(king, 1, 'Cross the bridge'));
    let row = rowOf(root, king.id);
    expect(row.dataset.change).toBe('out');
    expect(steps(row).map((s) => s.textContent)).toEqual(['Cross the bridge']);
    expect(steps(row)[0].classList.contains('leaving')).toBe(true);
    // Only this row plays.
    expect(root.querySelectorAll('[data-change]')).toHaveLength(1);

    advance(ROW_STEP_OUT_MS - 1);
    expect(rowOf(root, king.id).dataset.change).toBe('out');
    advance(1);
    row = rowOf(root, king.id);
    expect(row.dataset.change).toBe('in');
    expect(steps(row).map((s) => s.textContent)).toEqual(['Face the king']);
    expect(steps(row)[0].classList.contains('entering')).toBe(true);

    advance(ROW_STEP_IN_MS);
    row = rowOf(root, king.id);
    expect(row.dataset.change).toBeUndefined();
    expect(steps(row)[0].classList.contains('entering')).toBe(false);
    expect(steps(row)[0].textContent).toBe('Face the king');

    // A quest that became available has no step: its title comes in.
    const lute = view.next_up.find((r) => r.id === 'fixture-lute')!;
    bridge.pushRowChange(rowChangeFor(lute, 2, null));
    row = rowOf(root, lute.id);
    expect(row.dataset.change).toBe('in');
    expect(row.querySelector('.title')!.classList.contains('entering')).toBe(true);
    advance(ROW_STEP_IN_MS);
    expect(rowOf(root, lute.id).dataset.change).toBeUndefined();

    // The motion the owner was told: out by opacity, in from 0.2 rising 5 px, ease-out.
    expect([ROW_STEP_OUT_MS, ROW_STEP_IN_MS]).toEqual([150, 220]);
    expect(questRowSource).toMatch(/@keyframes row-step-out\s*{\s*to\s*{\s*opacity: 0;/);
    expect(questRowSource).toMatch(/@keyframes row-step-in\s*{\s*from\s*{\s*opacity: 0\.2;\s*transform: translateY\(5px\);/);
    expect(questRowSource).toMatch(/--row-ease: cubic-bezier\(0\.23, 1, 0\.32, 1\)/);
  });

  it('reduced_motion_change_is_instant', async () => {
    const view = fixtureView('panel-next-up', 'play');
    const king = view.next_up.find((r) => r.id === 'fixture-king')!;

    setReducedMotion(true);
    const { root, bridge } = await render(view);
    bridge.pushRowChange(rowChangeFor(king, 1, 'Cross the bridge'));
    const row = rowOf(root, king.id);
    // No phase at all: the new step is there at once, nothing leaves or enters.
    expect(row.dataset.change).toBeUndefined();
    expect(steps(row).map((s) => s.textContent)).toEqual(['Face the king']);
    expect(root.querySelector('.leaving, .entering')).toBeNull();
    // Still announced.
    expect(announced(root)).toEqual([rowChangeFor(king, 1, 'Cross the bridge').text]);
    // And the stylesheet drops the keyframes under the same setting.
    expect(questRowSource).toMatch(/@media \(prefers-reduced-motion: reduce\)\s*{\s*\.leaving,\s*\.entering\s*{\s*animation: none;/);
    rendered!.destroy();
    rendered = null;
    vi.useRealTimers();

    // Negative control: animations on, the same change plays.
    setReducedMotion(false);
    const on = await render(fixtureView('panel-next-up', 'play'));
    on.bridge.pushRowChange(rowChangeFor(king, 1, 'Cross the bridge'));
    expect(rowOf(on.root, king.id).dataset.change).toBe('out');
  });

  it('row_change_is_announced', async () => {
    // The act folded: the view carries no rows for it.
    const folded = fixtureView('panel-campaign', 'play');
    const king = folded.acts[0].quests.find((r) => r.id === 'fixture-king')!;
    folded.acts[0].expanded = false;
    folded.acts[0].quests = [];
    const { root, bridge } = await render(folded);

    // The region is there before anything changes, polite and empty.
    expect(announcer(root).getAttribute('aria-live')).toBe('polite');
    expect(announced(root)).toEqual([]);

    // A row not on screen: announced, not animated.
    const change = rowChangeFor(king, 1, 'Cross the bridge');
    expect(change.text).toMatch(/The King in the Mists/);
    bridge.pushRowChange(change);
    expect(announced(root)).toEqual([change.text]);
    expect(root.querySelectorAll('[data-change]')).toHaveLength(0);

    // Unfolding later shows the row as it is; the old change is not played.
    bridge.pushView(fixtureView('panel-campaign', 'play'));
    expect(rowOf(root, king.id)).not.toBeNull();
    expect(root.querySelectorAll('[data-change]')).toHaveLength(0);

    // The same sentence again is a new node, so it is read again.
    bridge.pushRowChange({ ...change, id: 2 });
    expect(announced(root)).toEqual([change.text, change.text]);
    expect(rowOf(root, king.id).dataset.change).toBe('out');

    // The region keeps only the last few nodes; it never grows with the session.
    for (let id = 3; id <= 10; id++) bridge.pushRowChange({ ...change, id, text: `Change ${id}.` });
    expect(announced(root)).toEqual(['Change 7.', 'Change 8.', 'Change 9.', 'Change 10.'].slice(-ANNOUNCED_KEEP));
  });

  it('second_change_restarts_cleanly', async () => {
    const view = fixtureView('panel-next-up', 'play');
    const king = view.next_up.find((r) => r.id === 'fixture-king')!;
    const { root, bridge } = await render(view);

    bridge.pushRowChange(rowChangeFor(king, 1, 'Step A'));
    const first = steps(rowOf(root, king.id))[0];
    advance(100);

    // Mid-way out, a second change: it starts again from its own beginning.
    bridge.pushRowChange(rowChangeFor(king, 2, 'Step B'));
    let row = rowOf(root, king.id);
    expect(row.dataset.change).toBe('out');
    expect(steps(row).map((s) => s.textContent)).toEqual(['Step B']);
    // A new element, so its animation runs from zero rather than from where the first stood.
    expect(steps(row)[0]).not.toBe(first);

    // The first change's timer is gone: 150 ms after it would have moved on, still out.
    advance(ROW_STEP_OUT_MS - 1);
    expect(rowOf(root, king.id).dataset.change).toBe('out');
    advance(1);
    expect(rowOf(root, king.id).dataset.change).toBe('in');

    // Mid-way in, a third: back to out, one step line, nothing queued.
    advance(100);
    bridge.pushRowChange(rowChangeFor(king, 3, 'Step C'));
    row = rowOf(root, king.id);
    expect(row.dataset.change).toBe('out');
    expect(steps(row).map((s) => s.textContent)).toEqual(['Step C']);
    advance(ROW_STEP_OUT_MS + ROW_STEP_IN_MS);
    row = rowOf(root, king.id);
    expect(row.dataset.change).toBeUndefined();
    expect(steps(row).map((s) => s.textContent)).toEqual(['Face the king']);
    // Nothing left to play.
    advance(10_000);
    expect(root.querySelectorAll('[data-change]')).toHaveLength(0);
    expect(vi.getTimerCount()).toBe(0);
  });

  it('new_character_label_shows', async () => {
    const provisional = (mode: 'play' | 'edit', cls: string): View => {
      const view = fixtureView('panel-campaign', mode);
      view.character = { ...view.character!, name: '', class: cls, level: 1, pinned: false, provisional: true };
      view.characters = view.characters.map((c) => ({ ...c, active: false }));
      return view;
    };

    // Play mode: "New character" as the name, level 1.
    let { root } = await render(provisional('play', ''));
    expect(root.querySelector('h1')?.textContent?.trim()).toBe(NEW_CHARACTER);
    expect(NEW_CHARACTER).toBe('New character');
    expect(root.querySelector('.meta')?.textContent?.trim()).toBe('Level 1');
    // Its Act 1 rows show at once.
    expect(root.querySelector('[data-quest="fixture-king"]')).not.toBeNull();
    rendered!.destroy();
    rendered = null;
    vi.useRealTimers();

    // A known class reads as for any character.
    ({ root } = await render(provisional('play', 'Witch')));
    expect(root.querySelector('.meta')?.textContent?.trim()).toBe('Witch 1');
    rendered!.destroy();
    rendered = null;
    vi.useRealTimers();

    // Edit mode: the picker button names it too.
    ({ root } = await render(provisional('edit', '')));
    const who = root.querySelector('button.who')!;
    expect(who.textContent?.trim()).toBe(NEW_CHARACTER);
    expect(who.getAttribute('aria-label')).toBe('New character: choose the character');
    rendered!.destroy();
    rendered = null;
    vi.useRealTimers();

    // Negative control: a named character shows its name, never the label.
    ({ root } = await render(fixtureView('panel-campaign', 'play')));
    expect(root.querySelector('h1')?.textContent?.trim()).toBe('Corisande_Fell');
    expect(root.textContent).not.toContain(NEW_CHARACTER);
  });
});
