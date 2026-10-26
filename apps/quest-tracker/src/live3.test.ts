import { flushSync } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import questDetailSource from './components/QuestDetail.svelte?raw';
import questRowSource from './components/QuestRow.svelte?raw';
import { rootProperties } from './contract';
import { type QuestRow, SCALE_MAX, SCALE_MIN, type View, viewProblems } from './ipc';
import { THUMB_PX, frameSender, pressValue, travelValue } from './slider';
import { CONTROL_SELECTOR, type Rendered, fakeBridge, fixtureView, readText, renderApp } from './test-support';

// The owner's fixes after Live 3, on the UI's side: the find-it tips under
// the step they help with, a skipped row in Next up with the step its
// tracked progress reached, Skip on a quest not picked up yet, and the
// opacity and scale sliders applying while they are dragged. Views are
// cloned fixtures; the animation frame is a queue the test runs by hand.

let rendered: Rendered | null = null;

afterEach(() => {
  rendered?.destroy();
  rendered = null;
  vi.unstubAllGlobals();
  setScreenRatio(1);
});

async function render(view: View) {
  const bridge = fakeBridge(view);
  rendered = await renderApp(bridge);
  return { root: rendered.root, bridge };
}

function close(): void {
  rendered?.destroy();
  rendered = null;
}

/** The animation frame as a queue: callbacks wait until `run`. */
function animationFrames() {
  const queue = new Map<number, FrameRequestCallback>();
  let next = 0;
  vi.stubGlobal('requestAnimationFrame', (cb: FrameRequestCallback) => {
    next += 1;
    queue.set(next, cb);
    return next;
  });
  vi.stubGlobal('cancelAnimationFrame', (id: number) => queue.delete(id));
  return {
    get waiting() {
      return queue.size;
    },
    run() {
      const due = [...queue.values()];
      queue.clear();
      for (const cb of due) cb(0);
      flushSync();
    },
  };
}

/** The webview's screen pixels per CSS pixel (its zoom times Windows' DPI scale). */
function setScreenRatio(ratio: number): void {
  Object.defineProperty(window, 'devicePixelRatio', { configurable: true, value: ratio });
}

function settingsView(patch: Partial<View['settings']> = {}): View {
  const view = fixtureView('settings', 'edit');
  view.frame = 'settings';
  Object.assign(view.settings, patch);
  return view;
}

function sliderOf(root: HTMLElement, label: string): HTMLInputElement {
  const row = [...root.querySelectorAll<HTMLElement>('.row')].find((r) => r.querySelector('.label')?.textContent?.trim() === label);
  expect(row, label).toBeTruthy();
  return row!.querySelector<HTMLInputElement>('input[type="range"]')!;
}

const valueText = (input: HTMLInputElement) => input.closest('.row')!.querySelector('.value')!.textContent;

function fire(input: HTMLInputElement, type: 'input' | 'change', value: string): void {
  input.value = value;
  input.dispatchEvent(new Event(type, { bubbles: true }));
  flushSync();
}

function pointer(target: Element, type: string, clientX: number): MouseEvent {
  const event = new MouseEvent(type, { bubbles: true, cancelable: true, button: 0, clientX });
  target.dispatchEvent(event);
  flushSync();
  return event;
}

/** The track's box, CSS pixels: 200 px of thumb travel from x = 100. */
const TRACK = { left: 100, width: 200 + THUMB_PX };
function placeTrack(input: HTMLInputElement): void {
  input.getBoundingClientRect = () =>
    ({ left: TRACK.left, width: TRACK.width, top: 0, height: 24, right: TRACK.left + TRACK.width, bottom: 24, x: TRACK.left, y: 0 }) as DOMRect;
}

/** The thumb's centre for `value`, in CSS pixels. */
const thumbAt = (value: number) => TRACK.left + THUMB_PX / 2 + ((value - SCALE_MIN) / (SCALE_MAX - SCALE_MIN)) * 200;

const updates = (bridge: { calls: Array<{ name: string; args: unknown }> }) =>
  bridge.calls.filter((c) => c.name === 'update_settings').map((c) => (c.args as { patch: unknown }).patch);

describe('the sliders after Live 3', () => {
  it('opacity_slider_applies_while_dragged', async () => {
    const frames = animationFrames();
    const { root, bridge } = await render(settingsView({ opacity: 0.8 }));
    const slider = sliderOf(root, 'Background opacity');

    // Dragging: each move shows at once, and goes out on the next frame.
    fire(slider, 'input', '0.7');
    expect(valueText(slider)).toBe('70%');
    expect(slider.style.getPropertyValue('--fill')).toBe('70%');
    expect(updates(bridge), 'nothing before the frame').toEqual([]);
    frames.run();
    expect(updates(bridge)).toEqual([{ opacity: 0.7 }]);

    // A view still carrying an older value does not pull the thumb back.
    fire(slider, 'input', '0.5');
    const echo = settingsView({ opacity: 0.7 });
    bridge.pushView(echo);
    expect(slider.value).toBe('0.5');
    expect(valueText(slider)).toBe('50%');
    frames.run();
    expect(updates(bridge)).toEqual([{ opacity: 0.7 }, { opacity: 0.5 }]);

    // The release sends the final value at once, without waiting for a frame.
    fire(slider, 'input', '0.4');
    fire(slider, 'change', '0.4');
    expect(updates(bridge)).toEqual([{ opacity: 0.7 }, { opacity: 0.5 }, { opacity: 0.4 }]);
    expect(frames.waiting).toBe(0);
    // The view catches up and takes over.
    bridge.pushView(settingsView({ opacity: 0.4 }));
    expect(valueText(slider)).toBe('40%');
    bridge.pushView(settingsView({ opacity: 0.25 }));
    expect(valueText(slider), 'the view leads again once it caught up').toBe('25%');
    close();

    // Negative control: a release with nothing new sends nothing twice.
    const again = await render(settingsView({ opacity: 0.8 }));
    const s2 = sliderOf(again.root, 'Background opacity');
    fire(s2, 'input', '0.6');
    frames.run();
    fire(s2, 'change', '0.6');
    expect(updates(again.bridge)).toEqual([{ opacity: 0.6 }]);
  });

  it('scale_slider_follows_pointer_travel', async () => {
    const frames = animationFrames();
    setScreenRatio(1);
    const { root, bridge } = await render(settingsView({ scale: 1 }));
    const slider = sliderOf(root, 'Scale');
    placeTrack(slider);

    // A press on the thumb keeps its value (no jump) and sends nothing.
    const down = pointer(slider, 'pointerdown', thumbAt(1));
    expect(down.defaultPrevented, "the range's own drag is off").toBe(true);
    frames.run();
    expect(updates(bridge)).toEqual([]);

    // 40 screen px of 200 is a fifth of the span (1.25): 1.25×, shown at once.
    pointer(slider, 'pointermove', thumbAt(1) + 40);
    expect(valueText(slider)).toBe('1.25×');
    frames.run();
    expect(updates(bridge)).toEqual([{ scale: 1.25 }]);

    // The shell re-zooms: CSS pixels grow 1.25×, the pointer's CSS x shrinks
    // for the same place on screen. 80 screen px from the press is 0.5.
    setScreenRatio(1.25);
    pointer(slider, 'pointermove', (thumbAt(1) + 80) / 1.25);
    frames.run();
    expect(updates(bridge)).toEqual([{ scale: 1.25 }, { scale: 1.5 }]);
    // What the range's CSS-pixel mapping would have read there instead: the value would chase the zoom.
    expect(pressValue(1, (thumbAt(1) + 80) / 1.25, TRACK, { min: SCALE_MIN, max: SCALE_MAX, step: 0.05 })).not.toBe(1.5);

    // Past the end it stops at the range's bound; the release sends the last value.
    pointer(slider, 'pointermove', 2000);
    pointer(slider, 'pointerup', 2000);
    expect(updates(bridge)).toEqual([{ scale: 1.25 }, { scale: 1.5 }, { scale: 2 }]);
    expect(frames.waiting).toBe(0);
    // After the release a move is not a drag.
    pointer(slider, 'pointermove', 100);
    frames.run();
    expect(updates(bridge)).toHaveLength(3);
    close();

    // Auto on: the slider rests at 1×. A press on the track jumps there, as a range does, and turns Auto off.
    setScreenRatio(1);
    const auto = await render(settingsView({ scale: 'auto' }));
    const s2 = sliderOf(auto.root, 'Scale');
    placeTrack(s2);
    pointer(s2, 'pointerdown', thumbAt(0.75));
    pointer(s2, 'pointerup', thumbAt(0.75));
    expect(updates(auto.bridge)).toEqual([{ scale: 0.75 }]);
    close();

    // Auto on, a press on the resting thumb with no move: nothing changes.
    const rest = await render(settingsView({ scale: 'auto' }));
    const s3 = sliderOf(rest.root, 'Scale');
    placeTrack(s3);
    pointer(s3, 'pointerdown', thumbAt(1) + 2);
    pointer(s3, 'pointerup', thumbAt(1) + 2);
    frames.run();
    expect(updates(rest.bridge)).toEqual([]);

    // The keyboard still steps it: the range's own input and change.
    fire(s3, 'input', '1.05');
    fire(s3, 'change', '1.05');
    expect(updates(rest.bridge)).toEqual([{ scale: 1.05 }]);
    close();

    // The mapping itself: travel over width across the span, on the step grid.
    const range = { min: SCALE_MIN, max: SCALE_MAX, step: 0.05 };
    expect(travelValue(1, 0, 200, range)).toBe(1);
    expect(travelValue(1, 16, 200, range)).toBe(1.1);
    expect(travelValue(1, -1000, 200, range)).toBe(0.75);
    expect(travelValue(1.33, 0, 0, range), 'no width: the start, snapped').toBe(1.35);
  });

  it('scale_slider_sends_at_most_one_update_per_frame', async () => {
    const frames = animationFrames();
    const { root, bridge } = await render(settingsView({ scale: 1 }));
    const slider = sliderOf(root, 'Scale');
    placeTrack(slider);
    pointer(slider, 'pointerdown', thumbAt(1));
    // Five moves inside one frame: one update, the newest.
    for (const dx of [8, 16, 24, 32, 40]) pointer(slider, 'pointermove', thumbAt(1) + dx);
    expect(frames.waiting).toBe(1);
    frames.run();
    expect(updates(bridge)).toEqual([{ scale: 1.25 }]);
    // A move that lands on the value already sent sends nothing more.
    pointer(slider, 'pointermove', thumbAt(1) + 41);
    frames.run();
    expect(updates(bridge)).toHaveLength(1);
    // One per frame across frames.
    pointer(slider, 'pointermove', thumbAt(1) + 48);
    frames.run();
    pointer(slider, 'pointermove', thumbAt(1) + 56);
    frames.run();
    expect(updates(bridge)).toEqual([{ scale: 1.25 }, { scale: 1.3 }, { scale: 1.35 }]);
    // The release sends a move still waiting for its frame, at once, and only once.
    pointer(slider, 'pointermove', thumbAt(1) + 64);
    pointer(slider, 'pointerup', thumbAt(1) + 64);
    frames.run();
    expect(updates(bridge)).toEqual([{ scale: 1.25 }, { scale: 1.3 }, { scale: 1.35 }, { scale: 1.4 }]);

    // A drag the system takes away (lost capture) ends like a release, and nothing sticks.
    pointer(slider, 'pointerdown', thumbAt(1.4));
    pointer(slider, 'pointermove', thumbAt(1.4) + 8);
    slider.dispatchEvent(new Event('lostpointercapture', { bubbles: true }));
    flushSync();
    expect(updates(bridge).at(-1)).toEqual({ scale: 1.45 });
    pointer(slider, 'pointermove', thumbAt(1.4) + 80);
    frames.run();
    expect(updates(bridge)).toHaveLength(5);
    // Closing the frame mid-drag still sends the last value moved to.
    pointer(slider, 'pointerdown', thumbAt(1.45));
    pointer(slider, 'pointermove', thumbAt(1.45) + 8);
    close();
    expect(updates(bridge).at(-1)).toEqual({ scale: 1.5 });
    expect(frames.waiting).toBe(0);
    // A touch drag stays a drag, not a pan.
    expect(await readText('./components/Settings.svelte')).toMatch(/\.slider\.tracked \{\s*touch-action: none;/);

    // The sender alone: never two sends in a frame, the last value always sent.
    const sent: number[] = [];
    const sender = frameSender<number>((v) => sent.push(v));
    sender.push(1);
    sender.push(2);
    sender.push(3);
    frames.run();
    sender.push(4);
    sender.flush();
    frames.run();
    expect(sent).toEqual([3, 4]);
    // A new gesture may send the value the last one ended on.
    sender.push(4);
    frames.run();
    sender.push(5);
    sender.cancel();
    frames.run();
    expect(sent, 'cancel drops the queued value').toEqual([3, 4, 4]);
  });
});

/** The `var(--…)` names a rule block reads, from `from` to its closing brace. */
function blockTokens(source: string, from: string): string[] {
  const start = source.indexOf(from);
  expect(start, from).toBeGreaterThan(-1);
  const block = source.slice(start, source.indexOf('}', start));
  return [...new Set([...block.matchAll(/var\((--[a-z0-9-]+)/g)].map((m) => m[1]))];
}

async function knownTokens(): Promise<Set<string>> {
  const tokens = rootProperties(await readText('../theme/tokens.css'));
  for (const name of rootProperties(await readText('./defaults.css'))) tokens.add(name);
  return tokens;
}

const rowOf = (root: HTMLElement, id: string) => root.querySelector<HTMLElement>(`[data-quest="${id}"]`)!;

describe('find-it tips', () => {
  it('row_shows_the_step_tip', async () => {
    for (const mode of ['play', 'edit'] as const) {
      const view = fixtureView('panel-next-up', mode);
      const king = view.next_up.find((r) => r.id === 'fixture-king')!;
      expect(king.current_tip).toBeTruthy();
      const { root } = await render(view);
      const row = rowOf(root, king.id);
      const step = row.querySelector('.step')!;
      const tip = row.querySelector('.tip');
      expect(step.textContent, mode).toBe(king.current_step);
      // Its own line, right under the step.
      expect(tip?.textContent, mode).toBe(king.current_tip);
      expect(step.nextElementSibling, mode).toBe(tip);
      // Negative control: a row with no tip has no tip line.
      const lute = view.next_up.find((r) => r.id === 'fixture-lute')!;
      expect(lute.current_tip).toBeNull();
      expect(rowOf(root, lute.id).querySelector('.tip'), mode).toBeNull();
      close();
    }

    // A tip with no step to sit under is not shown.
    const view = fixtureView('panel-next-up', 'play');
    const king = view.next_up.find((r) => r.id === 'fixture-king')!;
    king.current_step = null;
    const { root } = await render(view);
    expect(rowOf(root, king.id).querySelector('.tip')).toBeNull();
    close();

    // Secondary text, two lines at most, in existing tokens.
    expect(questRowSource).toMatch(/\.tip \{[^}]*color: var\(--color-text-secondary\)/);
    expect(questRowSource).toMatch(/\.tip \{[^}]*-webkit-line-clamp: 2;/);
    const known = await knownTokens();
    const uses = blockTokens(questRowSource, '.tip {');
    expect(uses.length).toBeGreaterThan(0);
    for (const name of uses) expect(known.has(name), name).toBe(true);

    // The IPC check knows the field, and nothing in place of it.
    const bad = fixtureView('panel-next-up');
    (bad.next_up[0] as unknown as Record<string, unknown>).current_tip = 3;
    expect(viewProblems(bad)).toEqual(['view.next_up[0].current_tip: expected a string']);
  });

  it('detail_shows_step_tips', async () => {
    for (const mode of ['play', 'edit'] as const) {
      const view = fixtureView('quest-detail', mode);
      const withTip = view.detail!.steps.filter((s) => s.tip);
      expect(withTip.map((s) => s.id)).toEqual(['face-the-king']);
      const { root } = await render(view);
      for (const step of view.detail!.steps) {
        const li = root.querySelector<HTMLElement>(`[data-step="${step.id}"]`)!;
        const tip = li.querySelector('.step-tip');
        if (step.tip) {
          expect(tip?.textContent, `${mode} ${step.id}`).toBe(step.tip);
          // Under the step's text, inside its body.
          expect(tip!.parentElement!.classList.contains('step-body')).toBe(true);
          expect(tip!.previousElementSibling?.classList.contains('step-text') || tip!.previousElementSibling?.classList.contains('step-title')).toBe(true);
        } else {
          expect(tip, `${mode} ${step.id}`).toBeNull();
        }
      }
      close();
    }
    const known = await knownTokens();
    for (const name of blockTokens(questDetailSource, '.step-tip {')) expect(known.has(name), name).toBe(true);
    expect(questDetailSource).toMatch(/\.step-tip \{[^}]*color: var\(--color-text-secondary\)/);
  });
});

describe('skipped quests stay tracked', () => {
  /** Next up with "Show skipped" on and the skipped pack leader listed after the unskipped rows, at its tracked step. */
  function nextUpWithSkipped(mode: 'play' | 'edit'): { view: View; skipped: QuestRow } {
    const view = fixtureView('panel-next-up', mode);
    view.settings.show_skipped = true;
    const campaign = fixtureView('panel-campaign', mode);
    const skipped = campaign.next_up.find((r) => r.status === 'skipped')!;
    expect(skipped, 'the campaign fixture lists a skipped quest in next up').toBeTruthy();
    skipped.current_step = 'Find the pack leader';
    skipped.current_tip = 'Fixture tip: more wolves, closer.';
    view.next_up.push(skipped);
    expect(viewProblems(view)).toEqual([]);
    return { view, skipped };
  }

  it('skipped_row_in_next_up_shows_its_step', async () => {
    for (const mode of ['play', 'edit'] as const) {
      const { view, skipped } = nextUpWithSkipped(mode);
      const { root } = await render(view);
      const row = rowOf(root, skipped.id);
      // Still a skipped row: its mark, muted and struck through like any finished row.
      expect(row.dataset.status).toBe('skipped');
      expect(row.classList.contains('finished')).toBe(true);
      expect(row.querySelector('.status-mark')).not.toBeNull();
      // With the step its tracked progress reached, and that step's tip.
      expect(row.querySelector('.step')?.textContent, mode).toBe('Find the pack leader');
      expect(row.querySelector('.tip')?.textContent, mode).toBe('Fixture tip: more wolves, closer.');
      close();
    }

    // Negative control: a skipped row with no tracked step shows none.
    const { view, skipped } = nextUpWithSkipped('play');
    view.next_up[view.next_up.length - 1].current_step = null;
    const { root } = await render(view);
    expect(rowOf(root, skipped.id).querySelector('.step, .tip')).toBeNull();
    close();

    // Muted and readable: the step and tip take the finished row's muted colour, and are not struck through.
    expect(questRowSource).toMatch(/\.row\.finished \.step,\s*\.row\.finished \.tip,[^{]*\{\s*color: var\(--color-text-muted\);/);
    expect(questRowSource).toMatch(/\.row\[data-status='skipped'\] \.name \{\s*text-decoration: line-through;/);
    expect(questRowSource).not.toMatch(/\.row\[data-status='skipped'\] \.(step|tip)/);
  });
});

describe('quest actions before the quest is picked up', () => {
  it('locked_quest_detail_offers_skip', async () => {
    const lockedView = (mode: 'play' | 'edit') => {
      const view = fixtureView('quest-detail', mode);
      view.detail!.row.status = 'locked';
      view.detail!.row.current_step = null;
      view.detail!.row.current_tip = null;
      return view;
    };

    const { root, bridge } = await render(lockedView('edit'));
    const buttons = [...root.querySelectorAll<HTMLButtonElement>('.actions button')].map((b) => b.textContent?.trim());
    expect(buttons).toContain('Skip');
    expect(buttons).not.toContain('Unskip');
    const skip = [...root.querySelectorAll<HTMLButtonElement>('.actions button')].find((b) => b.textContent?.trim() === 'Skip')!;
    skip.click();
    flushSync();
    expect(bridge.calls).toEqual([{ name: 'quest_action', args: { questId: 'fixture-king', action: 'skip' } }]);
    close();

    // A skipped one offers Unskip instead.
    const skippedView = lockedView('edit');
    skippedView.detail!.row.status = 'skipped';
    const skipped = await render(skippedView);
    const unskip = [...skipped.root.querySelectorAll<HTMLButtonElement>('.actions button')].find((b) => b.textContent?.trim() === 'Unskip');
    unskip!.click();
    flushSync();
    expect(skipped.bridge.calls).toEqual([{ name: 'quest_action', args: { questId: 'fixture-king', action: 'unskip' } }]);
    close();

    // Negative control: play mode offers no action at all.
    const play = await render(lockedView('play'));
    expect(play.root.querySelector('.actions')).toBeNull();
    expect(play.root.querySelectorAll(CONTROL_SELECTOR)).toHaveLength(0);
  });
});
