import { describe, expect, it } from 'vitest';
import { viewProblems } from './ipc';
import { FIXTURES, FIXTURE_NAMES, fixtureView } from './test-support';

describe('the TypeScript IPC types', () => {
  it('view_problems_catch_drift', () => {
    // One fixture per state the context's Fixtures list names, and nothing else.
    expect(Object.keys(FIXTURES).sort()).toEqual([...FIXTURE_NAMES].sort());
    for (const name of FIXTURE_NAMES) {
      expect(viewProblems(FIXTURES[name]), `${name}.json`).toEqual([]);
    }

    // Each fixture is the state its name says.
    const shape = Object.fromEntries(
      FIXTURE_NAMES.map((n) => {
        const v = fixtureView(n);
        return [n, [v.grab?.source ?? null, v.grab?.looks_like_item ?? null, v.status?.kind ?? null, v.overlay.visible]];
      }),
    );
    expect(shape).toEqual({
      empty: [null, null, null, false],
      'grabbed-item': ['shortcut', true, 'info', false],
      'grabbed-not-item': ['paste', false, 'error', false],
      'grab-refused': ['shortcut', true, 'error', false],
      'hotkey-taken': [null, null, 'error', false],
      'overlay-active': ['paste', true, 'info', true],
    });
    // The line count is the app's count of the normalised text's lines.
    for (const name of FIXTURE_NAMES) {
      const grab = fixtureView(name).grab;
      if (grab) expect(grab.line_count, name).toBe(grab.text.split('\n').length);
    }

    // Negative controls: the checker sees a drift in either direction.
    const extra = fixtureView('grabbed-item') as unknown as Record<string, unknown>;
    extra.surprise = 1;
    expect(viewProblems(extra)).toEqual(['view.surprise: not in the TypeScript types']);

    const missing = fixtureView('grabbed-item') as unknown as { grab: Record<string, unknown> };
    delete missing.grab.looks_like_item;
    expect(viewProblems(missing)).toEqual(['view.grab.looks_like_item: missing']);

    const badSource = fixtureView('grabbed-item') as unknown as { grab: Record<string, unknown> };
    badSource.grab.source = 'clipboard';
    expect(viewProblems(badSource)).toEqual(['view.grab.source: "clipboard" is not one of shortcut|paste']);

    const badCount = fixtureView('grabbed-item') as unknown as { grab: Record<string, unknown> };
    badCount.grab.line_count = -1;
    expect(viewProblems(badCount)).toEqual(['view.grab.line_count: expected an unsigned integer']);

    const badStatus = fixtureView('hotkey-taken') as unknown as { status: Record<string, unknown> };
    badStatus.status.kind = 'warn';
    expect(viewProblems(badStatus)).toEqual(['view.status.kind: "warn" is not one of info|error']);

    const badOverlay = fixtureView('overlay-active') as unknown as { overlay: Record<string, unknown> };
    badOverlay.overlay.visible = 'yes';
    badOverlay.overlay.position = { x: 0, y: 0 };
    expect(viewProblems(badOverlay)).toEqual([
      'view.overlay.visible: expected a boolean',
      'view.overlay.position: not in the TypeScript types',
    ]);

    const badUpdate = fixtureView('empty') as unknown as Record<string, unknown>;
    badUpdate.update_available = 13;
    expect(viewProblems(badUpdate)).toEqual(['view.update_available: expected a string']);
    expect(viewProblems(null)).toEqual(['view: expected an object']);
  });
});
