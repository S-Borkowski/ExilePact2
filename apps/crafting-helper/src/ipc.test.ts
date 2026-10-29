import { describe, expect, it } from 'vitest';
import { catalogueProblems, viewProblems } from './ipc';
import { CATALOGUE, FIXTURES, FIXTURE_NAMES, fixtureView } from './test-support';

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
        return [n, [v.grab?.source ?? null, v.grab?.looks_like_item ?? null, v.status?.kind ?? null, v.overlay.visible, v.craft.mode]];
      }),
    );
    expect(shape).toEqual({
      empty: [null, null, null, false, 'none'],
      'grabbed-item': ['shortcut', true, 'info', false, 'refused'],
      'grabbed-not-item': ['paste', false, 'error', false, 'refused'],
      'grab-refused': ['shortcut', true, 'error', false, 'refused'],
      'hotkey-taken': [null, null, 'error', false, 'none'],
      'overlay-active': ['paste', true, 'info', true, 'guiding'],
      'overlay-not-in-front': ['paste', true, 'info', false, 'guiding'],
      'craft-prefilled': ['paste', true, 'info', false, 'planning'],
      'craft-planning': ['paste', true, 'info', false, 'planning'],
      'craft-fallback': ['paste', true, 'info', false, 'planning'],
      'craft-no-route': ['paste', true, 'info', false, 'planning'],
      'craft-close-side': ['paste', true, 'info', false, 'planning'],
      'craft-side-full': ['paste', true, 'info', false, 'planning'],
      'craft-refused': ['paste', true, 'info', false, 'refused'],
      'craft-guiding': ['paste', true, 'info', false, 'guiding'],
      'craft-miss': ['paste', true, 'info', false, 'guiding'],
      'craft-miss-clean': ['paste', true, 'info', false, 'guiding'],
      'craft-replan': ['paste', true, 'error', false, 'guiding'],
      'craft-done': ['paste', true, 'info', false, 'guiding'],
      'craft-watching': ['paste', true, 'info', true, 'guiding'],
      'craft-screen-hit': ['paste', true, 'info', true, 'guiding'],
      'craft-screen-unexpected': ['paste', true, 'error', true, 'guiding'],
      'craft-watch-off': ['paste', true, 'info', true, 'guiding'],
      'craft-watch-unavailable': ['paste', true, 'info', true, 'guiding'],
    });
    // The overlay's content is there exactly while guiding, and it shows
    // only with content; the M5 views had no game in front, so it stays hidden.
    for (const name of FIXTURE_NAMES) {
      const v = fixtureView(name);
      expect(v.overlay.guide !== null, name).toBe(v.craft.mode === 'guiding');
      if (v.overlay.visible) expect(v.overlay.guide, name).not.toBeNull();
    }
    for (const name of ['craft-guiding', 'craft-miss', 'craft-replan', 'craft-done']) {
      expect(fixtureView(name).overlay.visible, name).toBe(false);
    }
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
    badOverlay.overlay.position = 'center';
    badOverlay.overlay.line = 'Next step: shown here while a craft is active.';
    expect(viewProblems(badOverlay)).toEqual([
      'view.overlay.visible: expected a boolean',
      'view.overlay.position: "center" is not one of top-left|top-center|top-right|bottom-left|bottom-center|bottom-right',
      'view.overlay.line: not in the TypeScript types',
    ]);

    // The overlay's content and the watch line, deep inside, and an entry's source.
    const badGuide = fixtureView('overlay-active');
    const guide = badGuide.overlay.guide! as unknown as Record<string, unknown>;
    guide.step_number = -1;
    (guide.watch as Record<string, unknown>).warning = 'yes';
    (guide.last as Record<string, unknown>).source = 'ocr';
    delete (badGuide.craft.guiding! as unknown as Record<string, unknown>).watch;
    expect(viewProblems(badGuide)).toEqual([
      'view.overlay.guide.step_number: expected an unsigned integer',
      'view.overlay.guide.last.source: "ocr" is not one of grab|screen',
      'view.overlay.guide.watch.warning: expected a boolean',
      'view.craft.guiding.watch: missing',
    ]);

    const badUpdate = fixtureView('empty') as unknown as Record<string, unknown>;
    badUpdate.update_available = 13;
    expect(viewProblems(badUpdate)).toEqual(['view.update_available: expected a string']);
    expect(viewProblems(null)).toEqual(['view: expected an object']);

    // The craft part: its fields, deep inside, and the mode naming the one part set.
    const badMode = fixtureView('craft-planning') as unknown as { craft: Record<string, unknown> };
    badMode.craft.mode = 'browsing';
    expect(viewProblems(badMode)).toEqual(['view.craft.mode: "browsing" is not one of none|refused|planning|guiding']);

    const twoParts = fixtureView('craft-planning') as unknown as { craft: Record<string, unknown> };
    twoParts.craft.refusal = 'stale';
    expect(viewProblems(twoParts)).toEqual(['view.craft.refusal: set in mode planning']);

    const noPart = fixtureView('craft-guiding') as unknown as { craft: Record<string, unknown> };
    noPart.craft.guiding = null;
    expect(viewProblems(noPart)).toEqual(['view.craft.guiding: missing in mode guiding']);

    const deep = fixtureView('craft-planning');
    (deep.craft.planning!.preview!.route!.steps[0] as unknown as Record<string, unknown>).odds = 12;
    (deep.craft.planning!.targets[0] as unknown as Record<string, unknown>).floor = -3;
    (deep.craft.planning!.blocked as unknown as Record<string, unknown>).limit = 200;
    expect(viewProblems(deep)).toEqual([
      'view.craft.planning.targets[0].floor: expected an unsigned integer',
      'view.craft.planning.blocked.limit: not in the TypeScript types',
      'view.craft.planning.preview.route.steps[0].odds: expected a string',
    ]);

    const badResult = fixtureView('craft-miss');
    (badResult.craft.guiding!.history[0] as unknown as Record<string, unknown>).result = 'near-miss';
    (badResult.craft.guiding! as unknown as Record<string, unknown>).status = 'paused';
    expect(viewProblems(badResult)).toEqual([
      'view.craft.guiding.status: "paused" is not one of active|stuck|replan|done',
      'view.craft.guiding.history[0].result: "near-miss" is not one of hit|miss|unexpected',
    ]);

    const noCraft = fixtureView('empty') as unknown as Record<string, unknown>;
    delete noCraft.craft;
    expect(viewProblems(noCraft)).toEqual(['view.craft: missing']);
  });

  it('catalogue_problems_catch_drift', () => {
    // The fixture `get_catalogue` answers with: groups with a heading, entries by name.
    expect(catalogueProblems(CATALOGUE)).toEqual([]);
    expect(CATALOGUE.length).toBeGreaterThan(1);
    for (const group of CATALOGUE) {
      expect(group.entries.length, group.label).toBeGreaterThan(0);
      const names = group.entries.map((e) => e.name);
      expect(names, group.label).toEqual([...names].sort((a, b) => a.localeCompare(b, 'en')));
    }

    // Negative controls.
    const drifted = JSON.parse(JSON.stringify(CATALOGUE)) as Array<Record<string, unknown>>;
    drifted[0].heading = drifted[0].label;
    delete drifted[0].label;
    (drifted[1].entries as Array<Record<string, unknown>>)[0].id = 7;
    expect(catalogueProblems(drifted)).toEqual([
      'catalogue[0].label: missing',
      'catalogue[0].heading: not in the TypeScript types',
      'catalogue[1].entries[0].id: expected a string',
    ]);
    expect(catalogueProblems({})).toEqual(['catalogue: expected a list']);
  });
});
