import { describe, expect, it } from 'vitest';
import { SEARCH_LIMIT, viewProblems } from './ipc';
import { FIXTURES, FIXTURE_NAMES, fixtureView } from './test-support';

describe('the TypeScript IPC types', () => {
  it('view_problems_catch_drift', () => {
    // One fixture per state the IPC contract lists, and nothing else.
    expect(Object.keys(FIXTURES).sort()).toEqual([...FIXTURE_NAMES].sort());

    for (const name of FIXTURE_NAMES) {
      expect(viewProblems(FIXTURES[name]), `${name}.json`).toEqual([]);
      expect(fixtureView(name).result.limit).toBe(SEARCH_LIMIT);
    }
    // Each fixture is the state its name says.
    const states = Object.fromEntries(FIXTURE_NAMES.map((n) => [n, fixtureView(n).result.state]));
    expect(states).toEqual({
      empty: 'empty',
      'ok-any': 'ok',
      'ok-all': 'ok',
      'too-long': 'too_long',
      unmatchable: 'unmatchable',
      'hotkey-taken': 'ok',
    });
    // The fixtures use every kind of value: both groups and kinds, an affix
    // name, a mod without a token, a too-long result with its parts.
    const mods = fixtureView('too-long').mods;
    expect(new Set(mods.map((m) => m.group))).toEqual(new Set(['waystone', 'desecrated']));
    expect(new Set(mods.map((m) => m.kind))).toEqual(new Set(['prefix', 'suffix']));
    expect(mods.some((m) => m.name !== null)).toBe(true);
    expect(mods.some((m) => m.token === null && m.why_not !== null)).toBe(true);
    const long = fixtureView('too-long').result;
    expect(long.length).toBeGreaterThan(SEARCH_LIMIT);
    expect(long.regex).toBe('');
    expect(long.length).toBe(long.parts.reduce((n, p) => n + p.length, 0) + long.parts.length - 1);
    const ok = fixtureView('ok-any').result;
    expect(ok.length).toBe(ok.regex.length);
    expect(ok.parts.map((p) => p.text).join(' ')).toBe(ok.regex);

    // Negative controls: the checker sees a drift in either direction.
    const extra = fixtureView('ok-any') as unknown as Record<string, unknown>;
    extra.surprise = 1;
    expect(viewProblems(extra)).toEqual(['view.surprise: not in the TypeScript types']);

    const missing = fixtureView('ok-any') as unknown as { mods: Array<Record<string, unknown>> };
    delete missing.mods[0].why_not;
    expect(viewProblems(missing)).toEqual(['view.mods[0].why_not: missing']);

    const badEnum = fixtureView('ok-any') as unknown as { selection: Record<string, unknown> };
    badEnum.selection.corruption = 'not_corrupted';
    expect(viewProblems(badEnum)).toEqual(['view.selection.corruption: "not_corrupted" is not one of any|corrupted|uncorrupted']);

    const badTier = fixtureView('ok-any') as unknown as { selection: Record<string, unknown> };
    badTier.selection.tier_max = 17;
    expect(viewProblems(badTier)).toEqual(['view.selection.tier_max: 17 is not a tier from 1 to 16']);

    const badPart = fixtureView('too-long') as unknown as { result: { parts: Array<Record<string, unknown>> } };
    badPart.result.parts[0].what = 'rarities';
    badPart.result.parts[1].length = -1;
    expect(viewProblems(badPart)).toEqual([
      'view.result.parts[0].what: "rarities" is not one of rarity|tier|want|avoid|corruption',
      'view.result.parts[1].length: expected an unsigned integer',
    ]);

    const badStatus = fixtureView('hotkey-taken') as unknown as { status: Record<string, unknown> };
    badStatus.status.kind = 'warn';
    expect(viewProblems(badStatus)).toEqual(['view.status.kind: "warn" is not one of info|error']);

    const nullToken = fixtureView('ok-any') as unknown as { mods: Array<Record<string, unknown>> };
    nullToken.mods[0].token = 3;
    expect(viewProblems(nullToken)).toEqual(['view.mods[0].token: expected a string']);
  });
});
