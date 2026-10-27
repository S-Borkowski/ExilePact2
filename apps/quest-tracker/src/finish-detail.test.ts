// Finish review F3 and F13 (quest detail): the leading step mark is the
// step's one toggle in edit mode and a plain mark in play mode, a step the log
// finished reads done in both, and the quest name is the frame's heading.
// Views are built here from a cloned fixture, never asserted by its names.

import { afterEach, describe, expect, it } from 'vitest';
import type { Mode, StepView, View } from './ipc';
import { CONTROL_SELECTOR, type Rendered, fakeBridge, fixtureView, readText, renderApp } from './test-support';

let rendered: Rendered | null = null;

afterEach(() => {
  rendered?.destroy();
  rendered = null;
});

const STEPS: StepView[] = [
  { id: 'step-pending', title: 'Pending step', text: 'Not started.', state: 'pending', ticked: false, inferred: false, tip: null },
  { id: 'step-current', title: 'Current step', text: '', state: 'current', ticked: false, inferred: false, tip: null },
  { id: 'step-log-done', title: 'Log done step', text: 'The log saw it.', state: 'done', ticked: false, inferred: false, tip: null },
  { id: 'step-hand-ticked', title: 'Hand ticked step', text: '', state: 'pending', ticked: true, inferred: false, tip: null },
];

function detailView(mode: Mode): View {
  const view = fixtureView('quest-detail', mode);
  view.frame = 'quest-detail';
  view.detail!.row.id = 'test-quest';
  view.detail!.row.name = 'Test quest name';
  view.detail!.steps = STEPS.map((step) => ({ ...step }));
  return view;
}

async function render(view: View) {
  const bridge = fakeBridge(view);
  rendered = await renderApp(bridge);
  return { root: rendered.root, bridge };
}

function stepItem(root: HTMLElement, id: string): HTMLElement {
  const el = root.querySelector<HTMLElement>(`.step[data-step="${id}"]`);
  expect(el, id).not.toBeNull();
  return el!;
}

/** What the leading mark draws: an icon id, or `box` for the empty box. */
function markOf(step: HTMLElement): string | null {
  const mark = step.querySelector('.mark');
  if (!mark) return null;
  if (mark.querySelector('.box')) return 'box';
  return mark.querySelector('.icon')?.getAttribute('data-icon') ?? null;
}

describe('finish review: quest detail', () => {
  it('detail_step_mark_is_the_only_toggle', async () => {
    const { root, bridge } = await render(detailView('edit'));
    expect(root.querySelector('.tick'), 'the trailing tick box is gone').toBeNull();

    for (const step of STEPS) {
      const li = stepItem(root, step.id);
      const controls = [...li.querySelectorAll<HTMLElement>(CONTROL_SELECTOR)];
      expect(controls, `${step.id} has one control`).toHaveLength(1);
      const toggle = controls[0];
      // The control is the leading mark itself, not a box beside the text.
      expect(toggle.classList.contains('mark'), step.id).toBe(true);
      expect(li.firstElementChild, `${step.id} mark leads`).toBe(toggle);
      expect(toggle.getAttribute('aria-pressed')).toBe(String(step.ticked));

      const before = bridge.calls.length;
      toggle.click();
      expect(bridge.calls.slice(before)).toEqual([
        { name: 'tick_step', args: { questId: 'test-quest', stepId: step.id, ticked: !step.ticked } },
      ]);
    }

    // The hit area is at least 24x24 (jsdom has no layout: read the rule).
    const source = await readText('./components/QuestDetail.svelte');
    const rule = source.match(/\n {2}button\.mark \{([^}]*)\}/);
    expect(rule, 'a button.mark rule').not.toBeNull();
    for (const prop of ['min-width', 'min-height']) {
      const px = rule![1].match(new RegExp(`${prop}:\\s*(\\d+)px`));
      expect(px, prop).not.toBeNull();
      expect(Number(px![1]), prop).toBeGreaterThanOrEqual(24);
    }
    expect(source).not.toMatch(/\.tick\b/);
  });

  it('log_done_step_reads_done_in_edit_mode', async () => {
    for (const mode of ['edit', 'play'] as const) {
      const { root } = await render(detailView(mode));
      // Done by the log, or ticked by hand: the check in a box, never an empty box.
      expect(markOf(stepItem(root, 'step-log-done')), `log done, ${mode}`).toBe('status-done');
      expect(markOf(stepItem(root, 'step-hand-ticked')), `hand ticked, ${mode}`).toBe('status-done');
      expect(stepItem(root, 'step-log-done').querySelectorAll('.box, .icon'), `log done, ${mode}: one mark`).toHaveLength(1);
      // Negative controls: a pending step is the empty box, the current one its glyph.
      expect(markOf(stepItem(root, 'step-pending')), `pending, ${mode}`).toBe('box');
      expect(markOf(stepItem(root, 'step-current')), `current, ${mode}`).toBe('status-in-progress');
      rendered!.destroy();
      rendered = null;
    }
  });

  it('play_mode_step_marks_are_not_controls', async () => {
    const play = await render(detailView('play'));
    const steps = play.root.querySelector('.steps')!;
    expect(steps.querySelectorAll(CONTROL_SELECTOR), 'play mode step controls').toHaveLength(0);
    for (const step of STEPS) expect(markOf(stepItem(play.root, step.id)), step.id).not.toBeNull();
    rendered!.destroy();
    rendered = null;

    // Negative control: the same view in edit mode has one toggle per step.
    const edit = await render(detailView('edit'));
    expect(edit.root.querySelector('.steps')!.querySelectorAll(CONTROL_SELECTOR)).toHaveLength(STEPS.length);
  });

  it('detail_title_is_a_heading', async () => {
    for (const mode of ['edit', 'play'] as const) {
      const { root } = await render(detailView(mode));
      const headings = [...root.querySelectorAll('.detail h2')];
      expect(headings.map((h) => h.textContent?.trim()), mode).toEqual(['Test quest name']);
      rendered!.destroy();
      rendered = null;
    }
  });
});
