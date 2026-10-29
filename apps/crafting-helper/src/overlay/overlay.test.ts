// The overlay (`overlay.html`, `ui-design.md` § "Overlay"): the guide's
// current step, where a hit and a miss go, the last outcome with its source
// and the screen watch's line, each as Rust wrote it in `view.overlay.guide`;
// stuck, replan and done say so. The panel is anchored to the edge the
// view's position names; the page sends nothing, decides nothing about
// whether it shows, and the preview opens on the active craft.

import { afterEach, describe, expect, it, vi } from 'vitest';
import { DEFAULT_OVERLAY_PREVIEW_VIEW, previewBridge } from '../bridge';
import type { View } from '../ipc';
import { FIXTURE_NAMES, type Rendered, fakeBridge, fixtureView, renderComponent, text } from '../test-support';
import Overlay from './Overlay.svelte';
import { previewScene } from './preview';

let mounted: Rendered[] = [];

afterEach(() => {
  for (const r of mounted) r.destroy();
  mounted = [];
  vi.restoreAllMocks();
});

const ready = (root: HTMLElement) => root.querySelector('.overlay')?.getAttribute('data-visible') !== 'none';

async function render(view: View) {
  const bridge = fakeBridge(view);
  const r = await renderComponent(Overlay, { bridge }, ready);
  mounted.push(r);
  return { ...r, bridge };
}

const texts = (root: Element, selector: string): string[] => [...root.querySelectorAll(selector)].map((el) => text(el));

/** Each row of the panel, top to bottom, by its class. */
const rows = (root: Element): string[] => [...root.querySelectorAll('.panel .row')].map((el) => el.classList[1]);

describe('the overlay', () => {
  it('overlay_renders_the_step_and_both_branches', async () => {
    const active = fixtureView('overlay-active');
    const g = active.overlay.guide!;
    expect(active.overlay.visible).toBe(true);
    expect(g.status).toBe('active');
    const { root, bridge } = await render(active);

    const overlay = root.querySelector('.overlay')!;
    expect(overlay.getAttribute('data-visible')).toBe('true');
    expect(overlay.getAttribute('data-position')).toBe('top-center');
    const panel = root.querySelector('.panel')!;
    expect(panel.getAttribute('data-status')).toBe('active');

    // All seven rows, in three groups, each as Rust sent it.
    expect(rows(root)).toEqual(['head', 'instruction', 'uses', 'branch', 'branch', 'last', 'watch']);
    expect(root.querySelectorAll('.panel .group')).toHaveLength(3);
    expect(text(root.querySelector('.head .status-word'))).toBe('Crafting');
    expect(text(root.querySelector('.head .route'))).toBe(g.route_name);
    expect(text(root.querySelector('.head .step'))).toBe('Step 2 · try 1');
    expect(text(root.querySelector('.instruction'))).toBe(g.instruction);
    expect(text(root.querySelector('.uses .label'))).toBe('Uses');
    expect(text(root.querySelector('.uses .names'))).toBe('Chaos Orb, Omen of Whittling');
    expect(text(root.querySelector('.uses .odds'))).toBe(g.odds);
    expect(texts(root, '.branch')).toEqual([g.on_hit, g.on_miss]);
    expect(texts(root, '.branch').map((b) => b.split(':')[0])).toEqual(['On a hit', 'On a miss']);
    expect(text(root.querySelector('.last'))).toBe(`Last: Hit from a grab ${g.last!.summary}`);
    expect(root.querySelector('.last .result')!.getAttribute('data-result')).toBe('hit');
    expect(text(root.querySelector('.watch'))).toBe(`Screen ${g.watch!.text}`);
    // Nothing on it to focus, select or click, and no live region.
    expect(root.querySelectorAll('button, input, select, a, [tabindex], [role="status"], [aria-live]')).toHaveLength(0);

    // No step taken yet: said in muted words, in the Last row's place.
    const fresh = fixtureView('overlay-active');
    fresh.overlay.guide = { ...fresh.overlay.guide!, last: null };
    bridge.pushView(fresh);
    expect(text(root.querySelector('.last'))).toBe('No step taken yet.');
    expect(root.querySelector('.last')!.classList.contains('empty')).toBe(true);

    // The rows follow the next view; the position anchors the panel.
    const moved = fixtureView('overlay-active');
    moved.overlay = { ...moved.overlay, position: 'bottom-right', guide: { ...moved.overlay.guide!, try_number: 3 } };
    bridge.pushView(moved);
    expect(overlay.getAttribute('data-position')).toBe('bottom-right');
    expect(text(root.querySelector('.head .step'))).toBe('Step 2 · try 3');

    // Hidden, with its content kept: the page draws what the view holds;
    // whether the window shows is the shell's to act on.
    const behind = fixtureView('overlay-not-in-front');
    expect(behind.overlay.visible).toBe(false);
    bridge.pushView(behind);
    expect(overlay.getAttribute('data-visible')).toBe('false');
    expect(text(root.querySelector('.instruction'))).toBe(behind.overlay.guide!.instruction);

    // Not guiding: no content, no panel.
    bridge.pushView(fixtureView('empty'));
    expect(fixtureView('empty').overlay.guide).toBeNull();
    expect(root.querySelector('.panel')).toBeNull();
    expect(overlay.getAttribute('data-visible')).toBe('false');

    // It never sends a command.
    expect(bridge.calls).toEqual([]);

    // Every view fixture renders, with a panel exactly while guiding.
    for (const name of FIXTURE_NAMES) {
      const view = fixtureView(name);
      const r = await render(view);
      expect(r.root.querySelector('.panel') !== null, name).toBe(view.overlay.guide !== null);
      expect(view.overlay.guide !== null, name).toBe(view.craft.mode === 'guiding');
      expect(r.bridge.calls, name).toEqual([]);
    }

    // Before any view: nothing drawn.
    const silent = await renderComponent(Overlay, { bridge: { ...fakeBridge(active), start: () => new Promise(() => {}) } }, () => true);
    mounted.push(silent);
    expect(silent.root.querySelector('.overlay')?.getAttribute('data-visible')).toBe('none');
    expect(silent.root.querySelector('.panel')).toBeNull();

    // The preview opens on the active craft, and paints a stand-in scene only when asked.
    expect(DEFAULT_OVERLAY_PREVIEW_VIEW).toBe('overlay-active');
    vi.spyOn(console, 'info').mockImplementation(() => {});
    const preview = await renderComponent(Overlay, { bridge: previewBridge('', DEFAULT_OVERLAY_PREVIEW_VIEW) }, ready);
    mounted.push(preview);
    expect(preview.root.querySelector('.overlay')?.getAttribute('data-visible')).toBe('true');
    expect(previewScene('?state=craft-done&scene=bright')).toBe('bright');
    expect(previewScene('?scene=dark')).toBe('dark');
    expect(previewScene('?scene=grey')).toBeNull();
    expect(previewScene('')).toBeNull();
  });

  it('overlay_renders_the_step_targets', async () => {
    // The overlay's step is Rust's instruction, which names the current
    // step's target modifiers (the guiding view's step line), the same
    // words the craft window's card shows; the page adds nothing to it.
    for (const name of ['overlay-active', 'craft-guiding', 'craft-watching']) {
      const view = fixtureView(name);
      const current = view.craft.guiding!.step!.step;
      const g = view.overlay.guide!;
      expect(current.targets.length, name).toBeGreaterThan(0);
      expect(g.instruction, name).toBe(current.instruction);
      const { root } = await render(view);
      const shown = text(root.querySelector('.panel .instruction'));
      expect(shown, name).toBe(g.instruction);
      for (const target of current.targets) expect(shown, name).toContain(target);
      expect(text(root.querySelector('.uses .names')), name).toBe(current.uses.join(', '));
    }
    // The last outcome's instruction is the history entry's, composed the same way.
    const active = fixtureView('overlay-active');
    expect(active.overlay.guide!.last!.instruction).toBe(active.craft.guiding!.history[0].instruction);
    expect(active.overlay.guide!.last!.instruction).toContain('+#% to Fire Resistance');
  });

  it('overlay_renders_the_watch_line', async () => {
    // Each watch state's line as sent, in the warning colour exactly when
    // Rust marks it; the same string the guiding card shows.
    const seen: Record<string, [string, string]> = {};
    for (const name of ['overlay-active', 'overlay-not-in-front', 'craft-watching', 'craft-screen-unexpected', 'craft-watch-off', 'craft-watch-unavailable', 'craft-guiding']) {
      const view = fixtureView(name);
      const watch = view.overlay.guide!.watch!;
      expect(watch, name).toEqual(view.craft.guiding!.watch);
      const { root } = await render(view);
      const row = root.querySelector('.watch')!;
      expect(text(row.querySelector('.label')), name).toBe('Screen');
      expect(text(row.querySelector('.watch-text')), name).toBe(watch.text);
      seen[name] = [row.getAttribute('data-warning')!, watch.text.split(':')[0]];
    }
    expect(seen).toEqual({
      'overlay-active': ['false', 'Watching'],
      'overlay-not-in-front': ['false', 'Paused'],
      'craft-watching': ['false', 'Watching'],
      'craft-screen-unexpected': ['true', 'The item changed in a way the guide did not expect'],
      'craft-watch-off': ['false', 'Screen reading is off'],
      'craft-watch-unavailable': ['true', "Can't read the screen (no English OCR)"],
      'craft-guiding': ['false', 'Screen reading is not available'],
    });

    // A screen read's outcome names its source.
    const hit = fixtureView('craft-screen-hit');
    const { root } = await render(hit);
    expect(text(root.querySelector('.last'))).toBe(`Last: Hit from the screen ${hit.overlay.guide!.last!.summary}`);
    expect(text(root.querySelector('.last .source'))).toBe('from the screen');

    // Miss and Unexpected carry the warning colour on their word.
    const miss = fixtureView('craft-miss');
    const m = await render(miss);
    expect(m.root.querySelector('.last .result')!.getAttribute('data-result')).toBe('miss');
    expect(text(m.root.querySelector('.last .result'))).toBe('Miss');
  });

  it('overlay_renders_stuck_replan_and_done', async () => {
    // Stuck: "Why:" and the first reason in place of the uses; the step,
    // both branches and the watch line stay.
    const stuck = fixtureView('overlay-active');
    stuck.overlay.guide = {
      ...stuck.overlay.guide!,
      status: 'stuck',
      uses: [],
      odds: null,
      why: 'no alternative of this step is usable',
      watch: { text: 'Paused: the guide is stuck.', warning: false },
    };
    const s = await render(stuck);
    expect(s.root.querySelector('.panel')!.getAttribute('data-status')).toBe('stuck');
    expect(text(s.root.querySelector('.head .status-word'))).toBe('Stuck');
    expect(rows(s.root)).toEqual(['head', 'instruction', 'why', 'branch', 'branch', 'last', 'watch']);
    expect(text(s.root.querySelector('.why'))).toBe('Why: no alternative of this step is usable');
    expect(text(s.root.querySelector('.watch .watch-text'))).toBe('Paused: the guide is stuck.');

    // Replan: what to do instead, why, then Last; no branches, no Screen row.
    const replan = fixtureView('craft-replan');
    const rg = replan.overlay.guide!;
    const r = await render(replan);
    expect(text(r.root.querySelector('.head .status-word'))).toBe('Replan');
    expect(r.root.querySelector('.head .step')).toBeNull();
    expect(rows(r.root)).toEqual(['head', 'instruction', 'why', 'last']);
    expect(text(r.root.querySelector('.instruction'))).toBe('End the craft in the app to plan again from this item.');
    expect(text(r.root.querySelector('.why'))).toBe(`Why: ${rg.why}`);
    expect(text(r.root.querySelector('.last'))).toBe(`Last: Unexpected from a grab ${rg.last!.summary}`);
    expect(r.root.querySelector('.last .result')!.getAttribute('data-result')).toBe('unexpected');

    // Done: says so, then the last outcome; nothing else.
    const done = fixtureView('craft-done');
    const d = await render(done);
    expect(text(d.root.querySelector('.head .status-word'))).toBe('Done');
    expect(rows(d.root)).toEqual(['head', 'instruction', 'last']);
    expect(text(d.root.querySelector('.instruction'))).toBe('The craft is done.');
    expect(text(d.root.querySelector('.last'))).toBe(`Last: Hit from a grab ${done.overlay.guide!.last!.summary}`);
    expect(d.root.querySelectorAll('.panel .group')).toHaveLength(2);
  });
});
