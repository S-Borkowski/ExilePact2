import { describe, expect, it } from 'vitest';
import brief from '../../../docs/quest-tracker/ui-design-brief.md?raw';
import { contractList } from './contract';
import { FRAMES, VIEW_SCHEMA, rowChangeProblems, viewProblems } from './ipc';
import { FIXTURES, fixtureView, rowChangeFor } from './test-support';

describe('the TypeScript IPC types', () => {
  it('ipc_types_match_viewmodel_fixtures', () => {
    const names = Object.keys(FIXTURES).sort();
    // One view per View frame, and nothing else (no toast since Live 1).
    expect(names).toEqual([...FRAMES].sort());

    for (const frame of FRAMES) {
      const view = FIXTURES[frame];
      expect(viewProblems(view), `${frame}.json`).toEqual([]);
      expect((view as { frame: string }).frame).toBe(frame);
      expect((view as { schema: number }).schema).toBe(VIEW_SCHEMA);
    }
    // The provisional flag is on every character the fixtures carry.
    const story = fixtureView('panel-campaign');
    expect(story.character?.provisional).toBe(false);

    // The View frames are the brief's frame ids less the two that are not a
    // View of their own (the two modes of one panel).
    const briefFrames = contractList(brief, 'Frame ids').filter((f) => !['mode-play', 'mode-edit'].includes(f));
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

    // A `RowChange` as the shell sends it passes; a drifted one does not.
    const change = rowChangeFor(story.acts[0].quests[0], 7, 'Find the camp', null);
    expect(rowChangeProblems(change)).toEqual([]);
    const badChange = { ...change, kind: 'quest-done', icon: 'status-done' } as unknown as Record<string, unknown>;
    expect(rowChangeProblems(badChange)).toEqual([
      'row_change.kind: "quest-done" is not one of available|step_done|quest_done',
      'row_change.icon: not in the TypeScript types',
    ]);
    const noFlag = fixtureView('panel-campaign') as unknown as { character: Record<string, unknown> };
    delete noFlag.character.provisional;
    expect(viewProblems(noFlag)).toEqual(['view.character.provisional: missing']);
  });
});
