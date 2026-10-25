// The "probably done" mark (owner, 2026-10-25, after launch 6): a row or a
// detail step the view flags `inferred` carries a dashed ring beside its
// status or step mark, with a tooltip and the same words as accessible text.
// It is not a control. Which rows and steps carry the flag is the view's to
// say; these views set it on a cloned fixture, and every test keeps a row or
// step without it beside the one with it.

import { afterEach, describe, expect, it } from 'vitest';
import type { Mode, View } from './ipc';
import { INFERRED_LABEL } from './labels';
import { CONTROL_SELECTOR, type Rendered, fakeBridge, fixtureView, renderApp } from './test-support';

let rendered: Rendered | null = null;

afterEach(() => {
  rendered?.destroy();
  rendered = null;
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

/** The campaign view with two done rows in act 1: `test-inferred` flagged, `test-own` not. */
function campaignView(mode: Mode): View {
  const view = fixtureView('panel-campaign', mode);
  const quests = view.acts[0].quests;
  expect(quests.length, 'the fixture has two act-1 rows to use').toBeGreaterThanOrEqual(2);
  Object.assign(quests[0], { id: 'test-inferred', status: 'done', inferred: true, needs_tick: false, current_step: null });
  Object.assign(quests[1], { id: 'test-own', status: 'done', inferred: false, needs_tick: false, current_step: null });
  for (const q of quests.slice(2)) q.inferred = false;
  view.acts[0].expanded = true;
  return view;
}

/** The detail view: two done steps, `step-inferred` flagged and `step-own` not, and a current one. */
function detailView(mode: Mode, rowInferred = false): View {
  const view = fixtureView('quest-detail', mode);
  view.frame = 'quest-detail';
  const detail = view.detail!;
  detail.row.id = 'test-quest';
  detail.row.status = rowInferred ? 'done' : 'in_progress';
  detail.row.inferred = rowInferred;
  detail.steps = [
    { id: 'step-inferred', title: 'Inferred step', text: 'Implied by a later step.', state: 'done', ticked: false, inferred: true },
    { id: 'step-own', title: 'Own step', text: 'The log saw it.', state: 'done', ticked: false, inferred: false },
    { id: 'step-current', title: 'Current step', text: '', state: 'current', ticked: false, inferred: false },
  ];
  return view;
}

function el(root: HTMLElement, selector: string): HTMLElement {
  const found = root.querySelector<HTMLElement>(selector);
  expect(found, selector).not.toBeNull();
  return found!;
}

const marks = (scope: Element) => scope.querySelectorAll<HTMLElement>('.inferred-mark');

describe('the "probably done" mark', () => {
  it('inferred_mark_on_row_and_step', async () => {
    for (const mode of ['play', 'edit'] as const) {
      // The row: after its status mark, which stays the done mark.
      const campaign = await render(campaignView(mode));
      const row = el(campaign.root, '.row[data-quest="test-inferred"]');
      expect(marks(row), `inferred row, ${mode}`).toHaveLength(1);
      const status = el(row, '.status-mark');
      expect(status.querySelector('[data-icon]')?.getAttribute('data-icon'), 'the status mark stays done').toBe('status-done');
      expect(status.compareDocumentPosition(marks(row)[0]) & Node.DOCUMENT_POSITION_FOLLOWING, 'after the status mark').toBeTruthy();
      expect(marks(row)[0].compareDocumentPosition(el(row, '.name')) & Node.DOCUMENT_POSITION_FOLLOWING, 'before the name').toBeTruthy();
      // Negative control: a row done by its own signal has no mark.
      expect(marks(el(campaign.root, '.row[data-quest="test-own"]')), `own row, ${mode}`).toHaveLength(0);
      close();

      // The step: after the step's done mark, which it does not replace.
      const detail = await render(detailView(mode));
      const step = el(detail.root, '.step[data-step="step-inferred"]');
      expect(marks(step), `inferred step, ${mode}`).toHaveLength(1);
      const mark = el(step, '.mark');
      expect(mark.querySelector('[data-icon]')?.getAttribute('data-icon'), 'the step mark stays done').toBe('status-done');
      expect(mark.nextElementSibling, 'right after the step mark').toBe(marks(step)[0]);
      expect(marks(el(detail.root, '.step[data-step="step-own"]')), `own step, ${mode}`).toHaveLength(0);
      expect(marks(el(detail.root, '.step[data-step="step-current"]')), `current step, ${mode}`).toHaveLength(0);
      // The detail's head carries the row's mark only when the row is flagged.
      expect(marks(el(detail.root, '.detail .head')), `head of an unflagged row, ${mode}`).toHaveLength(0);
      close();

      const flagged = await render(detailView(mode, true));
      const head = el(flagged.root, '.detail .head');
      expect(marks(head), `head of a flagged row, ${mode}`).toHaveLength(1);
      expect(el(head, '.status-mark').nextElementSibling).toBe(marks(head)[0]);
      close();
    }
  });

  it('inferred_step_offers_no_toggle', async () => {
    // Edit mode (owner, after Live 2: an inferred step "stays untickable"):
    // the flagged step has no control at all, and a click on its mark or its
    // ring sends nothing. A step the log saw, and one ticked by hand, keep
    // their one toggle, the leading mark.
    const view = detailView('edit');
    view.detail!.steps.push({ id: 'step-ticked', title: 'Ticked step', text: '', state: 'pending', ticked: true, inferred: false });
    const edit = await render(view);
    const inferred = el(edit.root, '.step[data-step="step-inferred"]');
    expect(inferred.querySelectorAll('button'), 'no button in the inferred step').toHaveLength(0);
    expect(inferred.querySelectorAll(CONTROL_SELECTOR), 'no control in the inferred step').toHaveLength(0);
    const before = edit.bridge.calls.length;
    el(inferred, '.mark').click();
    marks(inferred)[0].click();
    inferred.click();
    expect(edit.bridge.calls.slice(before), 'clicks on the inferred step').toEqual([]);
    expect(edit.bridge.calls.some((c) => c.name === 'tick_step')).toBe(false);
    // Its mark still reads as done, labelled, as in play mode.
    expect(el(inferred, '.mark [aria-label]').getAttribute('aria-label')).toBeTruthy();

    // Positive controls: the own and the ticked step keep the toggle, and it sends.
    for (const [id, ticked] of [
      ['step-own', true],
      ['step-ticked', false],
    ] as const) {
      const step = el(edit.root, `.step[data-step="${id}"]`);
      const controls = [...step.querySelectorAll<HTMLElement>(CONTROL_SELECTOR)];
      expect(controls.map((c) => c.className.split(' ')[0]), id).toEqual(['mark']);
      const at = edit.bridge.calls.length;
      el(step, 'button.mark').click();
      expect(edit.bridge.calls.slice(at), id).toEqual([{ name: 'tick_step', args: { questId: 'test-quest', stepId: id, ticked } }]);
    }
    // Mark done stays: the player's correction when the inference is wrong.
    const markDone = [...edit.root.querySelectorAll<HTMLButtonElement>('.actions button')].find((b) => b.textContent?.trim() === 'Mark done');
    expect(markDone, 'Mark done').toBeTruthy();
    close();

    // A row done by inference: Unmark done stays, the inferred step still has no toggle.
    const done = await render(detailView('edit', true));
    const unmark = [...done.root.querySelectorAll<HTMLButtonElement>('.actions button')].find((b) => b.textContent?.trim() === 'Unmark done');
    expect(unmark, 'Unmark done').toBeTruthy();
    unmark!.click();
    expect(done.bridge.calls).toEqual([{ name: 'quest_action', args: { questId: 'test-quest', action: 'unmark_done' } }]);
    expect(el(done.root, '.step[data-step="step-inferred"]').querySelectorAll('button')).toHaveLength(0);
    close();

    // Edit mode, the row: the ring adds no control; the row stays one button.
    const rows = await render(campaignView('edit'));
    for (const id of ['test-inferred', 'test-own']) {
      const row = el(rows.root, `.row[data-quest="${id}"]`);
      expect(row.tagName, id).toBe('BUTTON');
      expect(row.querySelectorAll(CONTROL_SELECTOR), `${id} holds no control`).toHaveLength(0);
      // `needs_tick` is the view's, unchanged by the flag: no manual marker here.
      expect(row.querySelectorAll('.manual'), id).toHaveLength(0);
    }
    close();

    // Play mode: nothing anywhere is a control, with the rings on screen.
    for (const view of [campaignView('play'), detailView('play', true)]) {
      const play = await render(view);
      expect(marks(play.root).length, view.frame).toBeGreaterThan(0);
      const controls = [...play.root.querySelectorAll(CONTROL_SELECTOR)].map((c) => c.outerHTML.slice(0, 80));
      expect(controls, `${view.frame} in play mode`).toEqual([]);
      close();
    }
  });

  it('inferred_mark_has_tooltip_and_label', async () => {
    expect(INFERRED_LABEL).toMatch(/^Probably done\b/);
    expect(INFERRED_LABEL).toMatch(/inferred from a later step the log recorded/);

    for (const view of [campaignView('play'), campaignView('edit'), detailView('play', true), detailView('edit', true)]) {
      const { root } = await render(view);
      const found = [...marks(root)];
      expect(found.length, `${view.frame} ${view.mode}`).toBeGreaterThan(0);
      for (const mark of found) {
        // The tooltip and the accessible text are the same words.
        expect(mark.getAttribute('title')).toBe(INFERRED_LABEL);
        expect(mark.getAttribute('aria-label')).toBe(INFERRED_LABEL);
        expect(mark.getAttribute('role')).toBe('img');
        // The drawing is decoration under that name, and a dashed ring, not
        // the done icon in another colour.
        const svg = mark.querySelector('svg');
        expect(svg?.getAttribute('aria-hidden')).toBe('true');
        expect(mark.querySelector('[data-icon]'), 'no icon id').toBeNull();
        const ring = svg?.querySelector('circle');
        expect(ring?.getAttribute('fill')).toBe('none');
        expect(ring?.getAttribute('stroke-dasharray')).toBeTruthy();
      }
      close();
    }

    // Negative control: an unflagged view carries no such words.
    const plain = fixtureView('panel-campaign', 'play');
    for (const act of plain.acts) for (const q of act.quests) q.inferred = false;
    for (const q of plain.next_up) q.inferred = false;
    const { root } = await render(plain);
    expect(marks(root)).toHaveLength(0);
    expect(root.querySelector(`[aria-label="${INFERRED_LABEL}"], [title="${INFERRED_LABEL}"]`)).toBeNull();
  });
});
