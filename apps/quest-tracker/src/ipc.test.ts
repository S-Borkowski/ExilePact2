import { describe, expect, it } from 'vitest';
import brief from '../../../docs/quest-tracker/ui-design-brief.md?raw';
import { contractList } from './contract';
import { FRAMES, VIEW_SCHEMA, toastProblems, viewProblems } from './ipc';
import { FIXTURES, fixtureToast, fixtureView } from './test-support';

describe('the TypeScript IPC types', () => {
  it('ipc_types_match_viewmodel_fixtures', () => {
    const names = Object.keys(FIXTURES).sort();
    // One view per View frame, plus the toast.
    expect(names).toEqual([...FRAMES, 'toast'].sort());

    for (const frame of FRAMES) {
      const view = FIXTURES[frame];
      expect(viewProblems(view), `${frame}.json`).toEqual([]);
      expect((view as { frame: string }).frame).toBe(frame);
      expect((view as { schema: number }).schema).toBe(VIEW_SCHEMA);
    }
    expect(toastProblems(FIXTURES.toast)).toEqual([]);

    // The View frames are the brief's frame ids less the three that are not
    // a View of their own (the two modes of one panel, and the toast event).
    const briefFrames = contractList(brief, 'Frame ids').filter((f) => !['mode-play', 'mode-edit', 'toast'].includes(f));
    expect([...FRAMES].sort()).toEqual(briefFrames.sort());

    // Negative controls: the checker does see a drift in either direction.
    const extra = fixtureView('panel-campaign') as unknown as Record<string, unknown>;
    extra.surprise = 1;
    expect(viewProblems(extra)).toEqual(['view.surprise: not in the TypeScript types']);

    const missing = fixtureView('quest-detail') as unknown as { detail: Record<string, unknown> };
    delete missing.detail.giver;
    expect(viewProblems(missing)).toEqual(['view.detail.giver: missing']);

    const badEnum = fixtureView('panel-campaign');
    (badEnum.acts[0].quests[0] as { status: string }).status = 'in-progress';
    expect(viewProblems(badEnum)).toHaveLength(1);
    expect(viewProblems(badEnum)[0]).toMatch(/status: "in-progress" is not one of/);

    const badToast = fixtureToast() as unknown as Record<string, unknown>;
    badToast.kind = 'quest-done';
    expect(toastProblems(badToast)).toHaveLength(1);
  });
});
