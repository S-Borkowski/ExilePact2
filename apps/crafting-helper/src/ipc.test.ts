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
        return [
          n,
          [v.grab?.source ?? null, v.grab?.looks_like_item ?? null, v.status?.kind ?? null, v.overlay.visible, v.craft.mode, v.trade.selection.open],
        ];
      }),
    );
    expect(shape).toEqual({
      empty: [null, null, null, false, 'none', false],
      'grabbed-item': ['shortcut', true, 'info', false, 'refused', false],
      'grabbed-not-item': ['paste', false, 'error', false, 'refused', false],
      'grab-refused': ['shortcut', true, 'error', false, 'refused', false],
      'hotkey-taken': [null, null, 'error', false, 'none', false],
      'overlay-active': ['paste', true, 'info', true, 'guiding', false],
      'overlay-not-in-front': ['paste', true, 'info', false, 'guiding', false],
      'craft-prefilled': ['paste', true, 'info', false, 'planning', false],
      'craft-planning': ['paste', true, 'info', false, 'planning', false],
      'craft-fallback': ['paste', true, 'info', false, 'planning', false],
      'craft-no-route': ['paste', true, 'info', false, 'planning', false],
      'craft-relaxation': ['paste', true, 'info', false, 'planning', false],
      'craft-blocker': ['paste', true, 'info', false, 'planning', false],
      'craft-essence-swap': ['paste', true, 'info', false, 'planning', false],
      'craft-close-side': ['paste', true, 'info', false, 'planning', false],
      'craft-last-desecration': ['paste', true, 'info', false, 'planning', false],
      'craft-side-full': ['paste', true, 'info', false, 'planning', false],
      'craft-refused': ['paste', true, 'info', false, 'refused', false],
      'craft-guiding': ['paste', true, 'info', false, 'guiding', false],
      'craft-miss': ['paste', true, 'info', false, 'guiding', false],
      'craft-miss-clean': ['paste', true, 'info', false, 'guiding', false],
      'craft-replan': ['paste', true, 'error', false, 'guiding', false],
      'craft-done': ['paste', true, 'info', false, 'guiding', false],
      'craft-watching': ['paste', true, 'info', true, 'guiding', false],
      'craft-screen-hit': ['paste', true, 'info', true, 'guiding', false],
      'craft-screen-unexpected': ['paste', true, 'error', true, 'guiding', false],
      'craft-watch-off': ['paste', true, 'info', true, 'guiding', false],
      'craft-watch-unavailable': ['paste', true, 'info', true, 'guiding', false],
      'trade-empty': ['paste', true, 'info', false, 'planning', true],
      'trade-picking': ['paste', true, 'info', false, 'planning', true],
      'trade-one-side': ['paste', true, 'info', false, 'planning', true],
      'trade-default-weight': ['paste', true, 'info', false, 'planning', true],
      'trade-refused': ['paste', true, 'error', false, 'planning', true],
      'trade-floor': ['paste', true, 'info', false, 'planning', true],
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

    // The trade part: missing, and drifts deep inside it.
    const noTrade = fixtureView('empty') as unknown as Record<string, unknown>;
    delete noTrade.trade;
    expect(viewProblems(noTrade)).toEqual(['view.trade: missing']);
    const badTrade = fixtureView('trade-picking');
    (badTrade.trade.prefixes[0] as unknown as Record<string, unknown>).weight_source = 'guess';
    (badTrade.trade.selection.picks[0] as unknown as Record<string, unknown>).floor = -1;
    (badTrade.trade.selection as unknown as Record<string, unknown>).url = 'x';
    (badTrade.trade as unknown as Record<string, unknown>).link = 7;
    (badTrade.trade as unknown as Record<string, unknown>).suggested_item_level = 6.5;
    delete (badTrade.trade.not_searchable[0] as unknown as Record<string, unknown>).reason;
    expect(viewProblems(badTrade)).toEqual([
      'view.trade.selection.picks[0].floor: expected an unsigned integer',
      'view.trade.selection.url: not in the TypeScript types',
      'view.trade.prefixes[0].weight_source: "guess" is not one of spawn|default',
      'view.trade.not_searchable[0].reason: missing',
      'view.trade.suggested_item_level: expected an unsigned integer',
      'view.trade.link: expected a string',
    ]);
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
