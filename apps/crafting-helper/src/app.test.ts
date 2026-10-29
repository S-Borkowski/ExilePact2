// The main window (`index.html`): every view fixture renders, a grab shows
// its raw text and line count, a paste becomes `grab_text` (and a paste into
// a field does not), the Settings button names an update, the Tauri bridge
// subscribes before it asks, and the preview reads its fixture from the
// query. Then the craft window (`ui-design.md` § "Craft window"): each
// `craft-*` fixture renders its mode, every control sends its command with
// the whole list, the page's own filters send nothing, and no page holds an
// outside address.

import { afterEach, describe, expect, it, vi } from 'vitest';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { emit } from '@tauri-apps/api/event';
import {
  DEFAULT_PREVIEW_VIEW,
  VIEW_EVENT,
  previewBridge,
  previewViewName,
  tauriBridge,
  untilManaged,
} from './bridge';
import type { View } from './ipc';
import {
  CATALOGUE,
  CRAFT_FIXTURE_NAMES,
  FIXTURE_NAMES,
  type Rendered,
  button,
  choose,
  click,
  fakeBridge,
  fixtureView,
  paste,
  renderComponent,
  renderView,
  setValue,
  text,
} from './test-support';
import App from './App.svelte';
import { OUTCOME_LABELS } from './craft/words';
import Overlay from './overlay/Overlay.svelte';

let mounted: Rendered[] = [];

async function render(view: View) {
  const r = await renderView(view);
  mounted.push(r);
  return r;
}

afterEach(() => {
  for (const r of mounted) r.destroy();
  mounted = [];
  vi.restoreAllMocks();
  clearMocks();
});

/** Tauri's rejection for a command whose `State` is not managed yet, word for word as the release exe logged it. */
const notManaged = (cmd: string): string =>
  `state not managed for field \`app\` on command \`${cmd}\`. You must call \`.manage()\` before using this command`;

/**
 * Mocks the shell as it answers before `.manage()`: each command in
 * `answers` rejects twice with Tauri's own message, then answers; the rest
 * answer nothing. Returns every command asked, in order.
 */
function notManagedTwice(answers: Record<string, () => unknown>): string[] {
  const asked: string[] = [];
  mockIPC(
    (cmd) => {
      asked.push(cmd);
      const answer = answers[cmd];
      if (!answer) return undefined;
      if (asked.filter((c) => c === cmd).length <= 2) throw notManaged(cmd);
      return answer();
    },
    { shouldMockEvents: true },
  );
  return asked;
}

describe('the main window', () => {
  it('placeholder_shows_raw_text_and_line_count', async () => {
    // No grab: how to grab, with the current hotkey, and no text block.
    const empty = fixtureView('empty');
    const { root, bridge } = await render(empty);
    expect(root.querySelector('.app')?.getAttribute('data-state')).toBe('empty');
    expect(text(root.querySelector('h1'))).toBe('Crafting Helper');
    expect(text(root.querySelector('h2'))).toBe('Last grabbed item');
    expect(root.querySelector('pre')).toBeNull();
    const howto = text(root.querySelector('.howto'));
    expect(howto).toContain('Ctrl+Alt+C');
    expect(howto).toContain('Ctrl+V');
    expect(text(root.querySelector('kbd.hotkey'))).toBe('Ctrl+Alt+X');
    expect(text(root.querySelector('[role="status"]'))).toBe('');

    // A grab: its line count, where it came from, the raw text exactly as sent.
    const grabbed = fixtureView('grabbed-item');
    bridge.pushView(grabbed);
    expect(root.querySelector('.app')?.getAttribute('data-state')).toBe('grabbed');
    expect(root.querySelector('.howto')).toBeNull();
    expect(text(root.querySelector('.count'))).toBe('10 lines');
    expect(text(root.querySelector('.source'))).toBe('from the hotkey');
    const raw = root.querySelector('pre.raw')!;
    expect(raw.textContent).toBe(grabbed.grab!.text);
    expect(raw.textContent!.split('\n')).toHaveLength(grabbed.grab!.line_count);
    expect(text(root.querySelector('[role="status"]'))).toBe('Grabbed an item: 10 lines.');
    expect(root.querySelector('.status')?.getAttribute('data-kind')).toBe('info');

    // A paste that is not an item: shown all the same, with its error marked "!".
    const notItem = fixtureView('grabbed-not-item');
    bridge.pushView(notItem);
    expect(text(root.querySelector('.count'))).toBe('3 lines');
    expect(text(root.querySelector('.source'))).toBe('from a paste');
    expect(root.querySelector('pre.raw')!.textContent).toBe('a shopping list\nmilk\neggs');
    const status = root.querySelector('[role="status"]')!;
    expect(status.getAttribute('aria-live')).toBe('polite');
    expect(text(status)).toBe(`!${notItem.status!.text}`);
    expect(status.querySelector('.mark')?.getAttribute('aria-hidden')).toBe('true');

    // One line is "1 line".
    bridge.pushView({ ...notItem, grab: { ...notItem.grab!, text: 'milk', line_count: 1 } });
    expect(text(root.querySelector('.count'))).toBe('1 line');

    // A refused grab keeps the last one and says why; a taken hotkey says so with no grab.
    const refused = await render(fixtureView('grab-refused'));
    expect(refused.root.querySelector('pre.raw')!.textContent).toBe(fixtureView('grab-refused').grab!.text);
    expect(text(refused.root.querySelector('[role="status"]'))).toBe('!Not grabbed: the clipboard is empty.');
    const taken = await render(fixtureView('hotkey-taken'));
    expect(taken.root.querySelector('pre')).toBeNull();
    expect(text(taken.root.querySelector('[role="status"]'))).toContain('taken by another program');

    // Every fixture renders, and none sends a command on its own.
    for (const name of FIXTURE_NAMES) {
      const r = await render(fixtureView(name));
      expect(r.root.querySelector('.app')?.getAttribute('data-state'), name).toBe(fixtureView(name).grab ? 'grabbed' : 'empty');
      expect(r.bridge.calls, name).toEqual([]);
    }
  });

  it('paste_sends_grab_text', async () => {
    const { root, bridge } = await render(fixtureView('empty'));
    const item = 'Item Class: Gloves\r\nRarity: Rare\r\n';

    // A paste anywhere in the window sends its text, as pasted: the app
    // normalises and validates it.
    const event = paste(root.querySelector('h2')!, item);
    expect(bridge.calls).toEqual([{ name: 'grab_text', args: { text: item } }]);
    expect(event.defaultPrevented).toBe(true);
    paste(document.body, 'second');
    expect(bridge.calls[1]).toEqual({ name: 'grab_text', args: { text: 'second' } });

    // A paste with no text still goes to the app, which refuses it in words.
    paste(document.body, null);
    paste(document.body, '');
    expect(bridge.calls.slice(2)).toEqual([
      { name: 'grab_text', args: { text: '' } },
      { name: 'grab_text', args: { text: '' } },
    ]);

    // Negative controls: a paste into an editable field is the field's.
    bridge.calls.length = 0;
    for (const html of ['<input type="text">', '<textarea></textarea>', '<div contenteditable="true"><span>x</span></div>']) {
      const host = document.createElement('div');
      host.innerHTML = html;
      root.querySelector('.app')!.appendChild(host);
      const field = host.querySelector('span') ?? host.firstElementChild!;
      const e = paste(field, item);
      expect(e.defaultPrevented, html).toBe(false);
      host.remove();
    }
    expect(bridge.calls).toEqual([]);

    // Nothing reads the clipboard but the paste event: no paste, no command.
    const readText = vi.fn();
    Object.defineProperty(navigator, 'clipboard', { value: { readText }, configurable: true });
    const fresh = await render(fixtureView('grabbed-item'));
    click(button(fresh.root, 'Settings'));
    window.dispatchEvent(new Event('focus'));
    expect(fresh.bridge.calls).toEqual([{ name: 'open_settings', args: {} }]);
    expect(readText).not.toHaveBeenCalled();
    delete (navigator as unknown as Record<string, unknown>).clipboard;

    // Once unmounted, the window no longer listens.
    const gone = await renderView(fixtureView('empty'));
    gone.destroy();
    paste(document.body, item);
    expect(gone.bridge.calls).toEqual([]);
  });

  it('settings_button_names_an_update', async () => {
    const { root, bridge } = await render(fixtureView('empty'));
    const settings = button(root, 'Settings')!;
    expect(settings.getAttribute('aria-label')).toBeNull();
    expect(root.querySelector('.update-mark')).toBeNull();
    click(settings);
    expect(bridge.calls).toEqual([{ name: 'open_settings', args: {} }]);

    bridge.pushView({ ...fixtureView('empty'), update_available: '1.3.0' });
    expect(text(root.querySelector('.update-mark'))).toBe('Update 1.3.0');
    expect(root.querySelector('button.settings')?.getAttribute('aria-label')).toBe('Settings, update 1.3.0 available');
  });

  it('tauri_bridge_subscribes_then_asks', async () => {
    const asked: string[] = [];
    mockIPC(
      (cmd, payload) => {
        asked.push(cmd);
        if (cmd === 'get_view') return fixtureView('empty');
        if (cmd === 'grab_text') {
          expect(payload).toEqual({ text: 'Item Class: Rings' });
          return fixtureView('grabbed-not-item');
        }
        return undefined;
      },
      { shouldMockEvents: true },
    );
    const views: View[] = [];
    const bridge = tauriBridge();
    const off = await bridge.start((v) => views.push(v));
    expect(asked).toContain('get_view');
    expect(views.map((v) => v.grab)).toEqual([null]);

    // A view the shell pushes on `ch:view` is delivered ...
    expect(VIEW_EVENT).toBe('ch:view');
    await emit(VIEW_EVENT, fixtureView('grabbed-item'));
    expect(views).toHaveLength(2);

    // ... and a command's own answer is not (the event carries the change).
    await bridge.command('grab_text', { text: 'Item Class: Rings' });
    expect(asked).toContain('grab_text');
    expect(views).toHaveLength(2);

    off();
    await emit(VIEW_EVENT, fixtureView('overlay-active'));
    expect(views).toHaveLength(2);
  });

  it('bridge_retries_until_the_app_state_is_ready', async () => {
    // The windows load while the shell's setup still runs: until the app's
    // state is managed, Tauri rejects every command with this message.
    const asked = notManagedTwice({ get_view: () => fixtureView('empty'), get_catalogue: () => CATALOGUE });
    const errors = vi.spyOn(console, 'error').mockImplementation(() => {});
    const r = await renderComponent(App, { bridge: tauriBridge() }, (root) => text(root).includes('Nothing grabbed yet'));
    mounted.push(r);
    expect(r.root.querySelector('.app')?.getAttribute('data-state')).toBe('empty');
    expect(text(r.root)).toContain('Nothing grabbed yet');
    expect(asked.filter((c) => c === 'get_view')).toHaveLength(3);
    expect(errors).not.toHaveBeenCalled();
  });

  it('catalogue_retries_until_the_app_state_is_ready', async () => {
    const asked = notManagedTwice({ get_catalogue: () => CATALOGUE });
    expect(await tauriBridge().catalogue()).toEqual(CATALOGUE);
    expect(asked.filter((c) => c === 'get_catalogue')).toHaveLength(3);
  });

  it('overlay_bridge_retries_until_the_app_state_is_ready', async () => {
    const asked = notManagedTwice({ get_view: () => fixtureView('overlay-active') });
    const errors = vi.spyOn(console, 'error').mockImplementation(() => {});
    const r = await renderComponent(
      Overlay,
      { bridge: tauriBridge() },
      (root) => root.querySelector('.overlay')?.getAttribute('data-visible') === 'true',
    );
    mounted.push(r);
    expect(r.root.querySelector('.overlay')?.getAttribute('data-visible')).toBe('true');
    expect(asked.filter((c) => c === 'get_view')).toHaveLength(3);
    expect(errors).not.toHaveBeenCalled();
  });

  it('bridge_does_not_retry_any_other_error', async () => {
    const asked: string[] = [];
    mockIPC(
      (cmd) => {
        asked.push(cmd);
        throw 'the view could not be built';
      },
      { shouldMockEvents: true },
    );
    await expect(tauriBridge().start(() => {})).rejects.toBe('the view could not be built');
    await expect(tauriBridge().catalogue()).rejects.toBe('the view could not be built');
    expect(asked).toEqual(['get_view', 'get_catalogue']);
  });

  it('bridge_gives_up_on_an_app_state_that_never_comes', async () => {
    let tries = 0;
    const call = async () => {
      tries++;
      throw notManaged('get_view');
    };
    await expect(untilManaged(call, { every: 5, giveUpAfter: 30 })).rejects.toBe(notManaged('get_view'));
    expect(tries).toBeGreaterThan(1);
  });

  it('preview_reads_the_state_from_the_query', async () => {
    expect(previewViewName('')).toBe(DEFAULT_PREVIEW_VIEW);
    expect(previewViewName('?state=hotkey-taken')).toBe('hotkey-taken');
    expect(previewViewName('?state=sideways')).toBe(DEFAULT_PREVIEW_VIEW);
    expect(previewViewName('?state=sideways', 'overlay-active')).toBe('overlay-active');
    for (const name of FIXTURE_NAMES) expect(previewViewName(`?state=${name}`)).toBe(name);

    vi.spyOn(console, 'info').mockImplementation(() => {});
    const bridge = previewBridge('?state=grabbed-not-item');
    const r = await renderComponent(App, { bridge }, (root) => root.querySelector('.app[data-state]') !== null);
    mounted.push(r);
    expect(text(r.root.querySelector('.count'))).toBe('3 lines');
    // A paste in the preview is logged, not acted on: the view stays the fixture's.
    paste(document.body, 'anything');
    expect(bridge.calls).toEqual([{ name: 'grab_text', args: { text: 'anything' } }]);
    expect(text(r.root.querySelector('.count'))).toBe('3 lines');
  });
});

/** The text of each element `selector` finds under `root`. */
const texts = (root: Element, selector: string): string[] => [...root.querySelectorAll(selector)].map((el) => text(el));
/** The checkbox whose label's name reads `name` (the label's text before any tag or hint). */
const checkbox = (root: Element, name: string): HTMLInputElement | undefined =>
  [...root.querySelectorAll<HTMLInputElement>('input[type="checkbox"]')].find(
    (box) => text(box.closest('label')?.querySelector('.name')) === name,
  );
/** The element whose `aria-label` is `name`. */
const labelled = <E extends Element = HTMLElement>(root: Element, name: string): E | null =>
  root.querySelector<E>(`[aria-label="${name}"]`);
/** Lets a focus move or a `toggle` event that waits a task happen. */
const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

/** Mounts the main window on the preview bridge, opened on the fixture `name` as `?state=` opens it. */
async function renderPreview(name: string) {
  vi.spyOn(console, 'info').mockImplementation(() => {});
  const bridge = previewBridge(`?state=${name}`);
  const r = await renderComponent(App, { bridge }, (root) => root.querySelector('.app[data-state]') !== null);
  mounted.push(r);
  return { ...r, bridge };
}

/** The fixtures in mode guiding. */
const GUIDING_FIXTURES = FIXTURE_NAMES.filter((name) => fixtureView(name).craft.mode === 'guiding');

/**
 * Every blueprint name a fixture's view carries: the guide's and the
 * overlay's, the previewed route's, the other routes' and the rejected
 * ones'. Rust keeps them for its own lines; the pages draw none.
 */
const BLUEPRINT_NAMES = [
  ...new Set(
    FIXTURE_NAMES.flatMap((name) => {
      const view = fixtureView(name);
      const preview = view.craft.planning?.preview ?? null;
      return [
        view.craft.guiding?.route_name,
        view.overlay.guide?.route_name,
        preview?.route?.name,
        ...(preview?.other_routes ?? []).map((other) => other.name),
        ...(preview?.rejected ?? []).map((line) => line.name),
      ].filter((n): n is string => typeof n === 'string' && n !== '');
    }),
  ),
];

/** How many times an outcome's word (hit, miss, unexpected) is said in `line`. */
const outcomeWords = (line: string): number => line.match(/\b(hit|miss|unexpected)\b/gi)?.length ?? 0;

describe('the craft window', () => {
  it('renders_every_craft_fixture', async () => {
    // Every fixture renders its mode, with the mode's first heading marked,
    // the header's two buttons, and sends nothing on its own.
    const headings: Record<string, string> = {};
    for (const name of FIXTURE_NAMES) {
      const view = fixtureView(name);
      const r = await render(view);
      expect(r.root.querySelector('.app')!.getAttribute('data-mode'), name).toBe(view.craft.mode);
      headings[name] = text(r.root.querySelector('.craft [data-mode-heading]'));
      expect(r.bridge.calls, name).toEqual([]);
      expect(button(r.root, 'Explore more crafting on Craft of Exile'), name).toBeDefined();
      expect(r.root.querySelector('button.settings'), name).not.toBeNull();
    }
    expect(headings).toEqual({
      empty: 'Last grabbed item',
      'grabbed-item': "Can't plan this item",
      'grabbed-not-item': "Can't plan this item",
      'grab-refused': "Can't plan this item",
      'hotkey-taken': 'Last grabbed item',
      'overlay-active': 'Doom Loop',
      'overlay-not-in-front': 'Doom Loop',
      'craft-prefilled': 'Doom Loop',
      'craft-planning': 'Doom Loop',
      'craft-fallback': 'Doom Loop',
      'craft-no-route': 'Doom Loop',
      'craft-close-side': 'Cord Snare',
      'craft-refused': "Can't plan this item",
      'craft-guiding': 'Doom Loop',
      'craft-miss': 'Doom Loop',
      'craft-miss-clean': 'Cord Snare',
      'craft-replan': 'Doom Loop',
      'craft-done': 'Doom Loop',
      'craft-watching': 'Doom Loop',
      'craft-screen-hit': 'Doom Loop',
      'craft-screen-unexpected': 'Doom Loop',
      'craft-watch-off': 'Doom Loop',
      'craft-watch-unavailable': 'Doom Loop',
    });

    // Refused: the reason as sent, how to grab another, the grabbed text open.
    const refusedView = fixtureView('craft-refused');
    const refused = await render(refusedView);
    expect(text(refused.root.querySelector('.refusal'))).toBe('the item is corrupted');
    expect(text(refused.root.querySelector('.grab-again p'))).toBe('Grab another item:');
    expect(texts(refused.root, '.grab-again li')).toEqual([
      'In the game, hover the item and press Ctrl+Alt+C.',
      `Press ${refusedView.hotkey}, or press Ctrl+V in this window.`,
    ]);
    const shown = refused.root.querySelector<HTMLDetailsElement>('details.grabbed-text')!;
    expect(shown.open).toBe(true);
    expect(text(shown.querySelector('summary'))).toBe('Grabbed text');
    expect(text(shown.querySelector('.grab-meta'))).toBe('18 lines, from a paste');
    expect(shown.querySelector('pre.raw')!.textContent).toBe(refusedView.grab!.text);
    expect(shown.querySelector('pre.raw')!.getAttribute('tabindex')).toBe('0');

    // Planning: the item, its modifiers by side with tier and tags, the
    // targets chosen, the choices grouped, the route and the Start bar.
    const planningView = fixtureView('craft-planning');
    const p = planningView.craft.planning!;
    const { root } = await render(planningView);
    const item = root.querySelector('.item-panel')!;
    expect(text(item.querySelector('.facts'))).toBe('Rare · Amethyst Ring · Rings · item level 76');
    for (const [side, label] of [
      ['prefix', 'Prefixes'],
      ['suffix', 'Suffixes'],
    ] as const) {
      const ofSide = p.item.modifiers.filter((m) => m.side === side);
      const list = item.querySelector(`ul.mods[data-side="${side}"]`);
      if (ofSide.length === 0) {
        expect(list, side).toBeNull();
        continue;
      }
      expect(text(item.querySelector(`#${list!.getAttribute('aria-labelledby')}`)), side).toBe(label);
      expect([...list!.querySelectorAll('li')].map((li) => texts(li, '.line')), side).toEqual(ofSide.map((m) => m.lines));
      expect(texts(list!, 'li .tier'), side).toEqual(ofSide.map((m) => (m.tier === null ? '—' : `T${m.tier}`)));
    }
    const closedText = item.querySelector<HTMLDetailsElement>('details.grabbed-text')!;
    expect(closedText.open).toBe(false);
    expect(closedText.querySelector('pre.raw')!.textContent).toBe(planningView.grab!.text);

    const targets = root.querySelector('.targets')!;
    expect(texts(targets, '.chosen li .target-text')).toEqual(p.targets.map((t) => t.text));
    expect(texts(targets, 'fieldset.choices > legend')).toEqual(['Prefixes · 0 chosen', 'Suffixes · 3 chosen', 'Desecrated · 0 chosen']);
    expect(targets.querySelectorAll('fieldset.choices[data-group="prefixes"] input[type="checkbox"]')).toHaveLength(
      p.choices.prefixes.length,
    );
    const fire = checkbox(targets, '+#% to Fire Resistance')!;
    expect(fire.checked).toBe(true);
    expect(text(fire.closest('label')!.querySelector('.best'))).toBe('best T2');
    // A chosen target's floor select carries Rust's labels, the chosen floor selected.
    const floor = labelled<HTMLSelectElement>(targets.querySelector('.chosen')!, 'Floor for +#% to Fire Resistance')!;
    const fireChoice = p.choices.suffixes.find((c) => c.mod_id === 'suffix-to-fire-resistance')!;
    expect([...floor.options].map((o) => o.textContent)).toEqual(fireChoice.floors.map((f) => f.label));
    expect(floor.selectedOptions[0].textContent).toBe('At least T3');
    expect(labelled(targets, 'Remove +#% to Fire Resistance')?.tagName).toBe('BUTTON');

    const route = root.querySelector('.route')!;
    const previewed = p.preview!.route!;
    expect(text(route.querySelector('h2'))).toBe('Route');
    expect(route.querySelector('.route-name')).toBeNull();
    expect(text(route.querySelector('.route-source'))).toBe(previewed.source);
    expect(route.querySelector('.route-notes')).toBeNull();
    const routeSelect = labelled<HTMLSelectElement>(route, 'Route')!;
    expect([...routeSelect.options].map((o) => o.textContent)).toEqual(['Route 1', 'Route 2', 'Route 3', 'Route 4', 'Route 5']);
    expect(texts(route, 'ol.steps > li .instruction')).toEqual(previewed.steps.map((s) => s.instruction));
    expect(texts(route, 'ol.steps > li .uses')).toEqual(previewed.steps.map((s) => `Uses ${s.uses.join(', ')}`));
    expect(route.querySelector('.fallback')).toBeNull();
    const blocked = root.querySelector<HTMLDetailsElement>('details.blocked')!;
    expect(text(blocked.querySelector('summary'))).toBe('Blocked for this craft: 0');
    expect(blocked.open).toBe(false);
    const start = button(root, 'Start crafting')!;
    expect(start.disabled).toBe(false);
    expect(start.classList.contains('primary')).toBe(true);
    expect(root.querySelector('.start-bar .why')).toBeNull();

    // Fallback: the line under its label, the disclosure open, the entry tagged.
    const fallback = await render(fixtureView('craft-fallback'));
    const whittle = fallback.root.querySelectorAll('ol.steps > li')[1];
    expect(text(whittle.querySelector('.fallback .label'))).toBe('Fallback');
    expect(text(whittle.querySelector('.fallback .line'))).toBe('Perfect Exalted Orb instead of Chaos Orb: Omen of Whittling blocked');
    const fbBlocked = fallback.root.querySelector<HTMLDetailsElement>('details.blocked')!;
    expect(fbBlocked.open).toBe(true);
    expect(text(fbBlocked.querySelector('summary'))).toBe('Blocked for this craft: 1');
    expect(text(fbBlocked.querySelector('.help'))).toBe('Changes here apply to this craft only; Settings keeps your usual list.');
    const omen = checkbox(fbBlocked, 'Omen of Whittling')!;
    expect(omen.checked).toBe(true);
    expect(text(omen.closest('label')!.querySelector('.tag'))).toBe('this craft');

    // No route: one line per rejection in Rust's order; Start disabled and why.
    const noRouteView = fixtureView('craft-no-route');
    const noRoute = await render(noRouteView);
    expect(text(noRoute.root.querySelector('.no-route'))).toBe('No route reaches these targets.');
    expect(texts(noRoute.root, 'ul.rejected li')).toEqual(
      noRouteView.craft.planning!.preview!.rejected.map((r) => r.reason),
    );
    expect(noRoute.root.querySelector<HTMLDetailsElement>('details.blocked')!.open).toBe(true);
    expect(button(noRoute.root, 'Start crafting')!.disabled).toBe(true);
    expect(text(noRoute.root.querySelector('.start-bar .why'))).toBe('No route to start.');

    // No target: no preview, and the empty prompts say so.
    const bare = fixtureView('craft-planning');
    bare.craft.planning = { ...bare.craft.planning!, targets: [], preview: null, can_accept: false };
    const none = await render(bare);
    expect(text(none.root.querySelector('.route .no-target'))).toBe('Pick a target to see a route.');
    expect(none.root.querySelector('ol.steps')).toBeNull();
    expect(text(none.root.querySelector('.chosen-empty'))).toBe('Pick the modifiers you want from the list below.');
    expect(button(none.root, 'Clear targets')!.disabled).toBe(true);
    expect(text(none.root.querySelector('.start-bar .why'))).toBe('Pick a target first.');

    // No explicit modifier, and a modifier with no tier, said in words.
    const plain = fixtureView('craft-planning');
    plain.craft.planning!.item.modifiers = [];
    const empty = await render(plain);
    expect(text(empty.root.querySelector('.item-panel .no-mods'))).toBe('No explicit modifiers.');
    const tagged = fixtureView('craft-planning');
    const mod0 = tagged.craft.planning!.item.modifiers[0];
    tagged.craft.planning!.item.modifiers[0] = { ...mod0, tier: null, fractured: true, meets_target: true };
    const tags = await render(tagged);
    const first = tags.root.querySelector('ul.mods li')!;
    expect(text(first.querySelector('.tier'))).toBe('—');
    expect(first.querySelector('.tier')!.getAttribute('aria-label')).toBe('no tier');
    expect(texts(first, '.tag')).toEqual(['fractured', 'target']);
  });

  it('planning_shows_the_items_own_mods_ticked', async () => {
    // Right after a grab (`craft-prefilled`): the item's own explicit
    // modifiers are the chosen targets, each at the tier it has, ticked in
    // their group and marked met on the item. The page sent nothing for it:
    // the list is Rust's.
    const view = fixtureView('craft-prefilled');
    const p = view.craft.planning!;
    expect(p.targets.map((t) => [t.text, t.side, t.floor])).toEqual([
      ['+# to maximum Mana', 'prefix', 5],
      ['+# to Strength', 'suffix', 5],
    ]);
    expect(p.item.modifiers.map((m) => [m.side, m.tier, m.meets_target])).toEqual([
      ['prefix', 5, true],
      ['suffix', 5, true],
    ]);
    const { root, bridge } = await render(view);
    const targets = root.querySelector('.targets')!;
    expect(texts(targets, '.chosen li .target-text')).toEqual(['+# to maximum Mana', '+# to Strength']);
    expect(texts(targets, 'fieldset.choices > legend')).toEqual(['Prefixes · 1 chosen', 'Suffixes · 1 chosen', 'Desecrated · 0 chosen']);
    for (const name of ['+# to maximum Mana', '+# to Strength']) {
      expect(checkbox(targets, name)!.checked, name).toBe(true);
      const floor = labelled<HTMLSelectElement>(targets.querySelector('.chosen')!, `Floor for ${name}`)!;
      expect(floor.selectedOptions[0].textContent, name).toBe('At least T5');
    }
    for (const li of root.querySelectorAll('.item-panel ul.mods li')) {
      expect(texts(li, '.tag')).toContain('target');
    }
    expect(button(root, 'Clear targets')!.disabled).toBe(false);
    expect(bridge.calls).toEqual([]);

    // Negative control: with the player's own list (resistances), the
    // item's modifiers are neither ticked nor marked as targets.
    const planning = await render(fixtureView('craft-planning'));
    const other = planning.root.querySelector('.targets')!;
    expect(checkbox(other, '+# to maximum Mana')!.checked).toBe(false);
    expect(checkbox(other, '+# to Strength')!.checked).toBe(false);
    for (const li of planning.root.querySelectorAll('.item-panel ul.mods li')) {
      expect(texts(li, '.tag')).not.toContain('target');
    }
  });

  it('route_preview_lists_targets_and_the_craft_not_the_notes', async () => {
    // The Route panel says what is targeted and what to do: the chosen
    // targets with their floors (Rust's labels) above the steps, the route's
    // source; never the blueprint's name or notes, which the view no longer
    // carries for the page.
    for (const name of ['craft-planning', 'craft-prefilled', 'craft-fallback']) {
      const view = fixtureView(name);
      const p = view.craft.planning!;
      const previewed = p.preview!.route!;
      expect(Object.keys(previewed).sort(), name).toEqual(['blueprint', 'name', 'source', 'steps']);
      const { root } = await render(view);
      const route = root.querySelector('.route')!;
      expect(route.querySelector('.route-name'), name).toBeNull();
      expect(text(route.querySelector('.route-source')), name).toBe(previewed.source);
      expect(route.querySelector('.route-notes'), name).toBeNull();

      const list = route.querySelector('ul.route-targets')!;
      expect(text(route.querySelector(`#${list.getAttribute('aria-labelledby')}`)), name).toBe('Targets');
      expect(texts(list, 'li .target-text'), name).toEqual(p.targets.map((t) => t.text));
      const floors = new Map(
        [...p.choices.prefixes, ...p.choices.suffixes, ...p.choices.desecrated].map((c) => [c.mod_id, c.floors]),
      );
      expect(texts(list, 'li .floor'), name).toEqual(
        p.targets.map((t) => floors.get(t.mod_id)!.find((f) => f.floor === t.floor)!.label),
      );
      // The targets come before the steps.
      const steps = route.querySelector('ol.steps')!;
      expect(list.compareDocumentPosition(steps) & Node.DOCUMENT_POSITION_FOLLOWING, name).toBeTruthy();
      expect(texts(steps, ':scope > li .instruction'), name).toEqual(previewed.steps.map((s) => s.instruction));
    }
    expect(texts((await render(fixtureView('craft-planning'))).root, '.route-targets .floor')).toEqual([
      'At least T3',
      'At least T3',
      'At least T3',
    ]);

    // No target: no preview, so no target list either.
    const bare = fixtureView('craft-planning');
    bare.craft.planning = { ...bare.craft.planning!, targets: [], preview: null, can_accept: false };
    const none = await render(bare);
    expect(none.root.querySelector('.route-targets')).toBeNull();
    expect(text(none.root.querySelector('.route .no-target'))).toBe('Pick a target to see a route.');
  });

  it('route_preview_does_not_name_the_blueprint', async () => {
    // The blueprint's name is the app's recipe, not the player's: the Route
    // panel has no route-name line, its chooser labels the routes `Route 1`,
    // `Route 2`, ... in Rust's order (the previewed one first) with the
    // blueprint id as each option's value, and a rejection is its reason
    // alone.
    expect(BLUEPRINT_NAMES.length).toBeGreaterThan(2);
    for (const name of ['craft-prefilled', 'craft-planning', 'craft-fallback', 'craft-no-route']) {
      const { root } = await renderPreview(name);
      const route = root.querySelector('.route')!;
      expect(route, name).not.toBeNull();
      expect(route.querySelector('.route-name'), name).toBeNull();
      for (const blueprintName of BLUEPRINT_NAMES) expect(route.textContent, `${name}: ${blueprintName}`).not.toContain(blueprintName);
    }

    // The chooser, shown only with other routes.
    const p = fixtureView('craft-planning').craft.planning!.preview!;
    expect(p.other_routes.length).toBeGreaterThan(0);
    const planning = await renderPreview('craft-planning');
    const select = labelled<HTMLSelectElement>(planning.root.querySelector('.route')!, 'Route')!;
    expect([...select.options].map((o) => o.textContent)).toEqual(['Route 1', 'Route 2', 'Route 3', 'Route 4', 'Route 5']);
    expect([...select.options].map((o) => o.value)).toEqual([p.route!.blueprint, ...p.other_routes.map((o) => o.blueprint)]);
    expect(select.value).toBe(p.route!.blueprint);
    // A route with no other (every fixture's item now has several): the
    // chooser is left out.
    const alone = fixtureView('craft-fallback');
    alone.craft.planning!.preview!.other_routes = [];
    const single = await render(alone);
    expect(labelled(single.root.querySelector('.route')!, 'Route')).toBeNull();

    // No route: each rejection is its reason, in Rust's order.
    const rejected = fixtureView('craft-no-route').craft.planning!.preview!.rejected;
    expect(rejected.length).toBeGreaterThan(0);
    const noRoute = await renderPreview('craft-no-route');
    expect(texts(noRoute.root, 'ul.rejected li')).toEqual(rejected.map((r) => r.reason));
  });

  it('step_lines_name_their_target_mods', async () => {
    // Each step that places a target names its target modifiers: in the
    // route's step list and on the current step's card, as Rust sent them,
    // under an instruction Rust composed that names them too.
    const view = fixtureView('craft-planning');
    const steps = view.craft.planning!.preview!.route!.steps;
    expect(steps.every((s) => s.targets.length > 0)).toBe(true);
    const { root } = await render(view);
    const shown = [...root.querySelectorAll('.route ol.steps > li')];
    expect(shown).toHaveLength(steps.length);
    steps.forEach((step, i) => {
      expect(text(shown[i].querySelector('.step-targets .label')), step.id).toBe('Targets');
      expect(texts(shown[i], '.step-targets .target'), step.id).toEqual(step.targets);
      for (const target of step.targets) expect(text(shown[i].querySelector('.instruction')), step.id).toContain(target);
    });

    // A step that places none (the quality steps of `craft-prefilled`'s
    // route) shows its blueprint text and no Targets line.
    const prefilled = fixtureView('craft-prefilled');
    const plain = prefilled.craft.planning!.preview!.route!.steps;
    expect(plain.every((s) => s.targets.length === 0)).toBe(true);
    const pr = await render(prefilled);
    expect(texts(pr.root, '.route ol.steps > li .instruction')).toEqual(plain.map((s) => s.instruction));
    expect(pr.root.querySelector('.route .step-targets')).toBeNull();

    // The current step's card.
    const guiding = fixtureView('craft-guiding');
    const current = guiding.craft.guiding!.step!.step;
    expect(current.targets).toEqual(['+#% to Cold Resistance', '+#% to Lightning Resistance']);
    const gr = await render(guiding);
    const card = gr.root.querySelector('.guide .card')!;
    expect(text(card.querySelector('.instruction'))).toBe(current.instruction);
    expect(texts(card, '.step-targets .target')).toEqual(current.targets);
    for (const target of current.targets) expect(text(card.querySelector('.instruction'))).toContain(target);
  });

  it('target_filter_hides_rows_without_changing_targets', async () => {
    const view = fixtureView('craft-planning');
    const choices = view.craft.planning!.choices;
    const { root, bridge } = await render(view);
    const targets = root.querySelector('.targets')!;
    const visible = () =>
      [...targets.querySelectorAll<HTMLElement>('fieldset.choices li')]
        .filter((li) => !li.hidden && !li.closest('fieldset')!.hidden)
        .map((li) => text(li.querySelector('.name')));
    const all = visible();
    expect(all).toHaveLength(choices.prefixes.length + choices.suffixes.length + choices.desecrated.length);

    const filter = labelled<HTMLInputElement>(targets, 'Filter modifiers')!;
    expect(filter.tagName).toBe('INPUT');
    setValue(filter, 'RESISTANCE');
    const narrowed = visible();
    expect(narrowed.length).toBeGreaterThan(2);
    expect(narrowed.length).toBeLessThan(all.length);
    for (const row of narrowed) expect(row.toLowerCase()).toContain('resistance');

    // The chosen targets and their checks stay as the view says; nothing is sent.
    expect(texts(targets, '.chosen li .target-text')).toEqual(view.craft.planning!.targets.map((t) => t.text));
    expect(checkbox(targets, '+#% to Cold Resistance')!.checked).toBe(true);
    expect(bridge.calls).toEqual([]);

    // No match says so with the text typed, and hides every group; clearing
    // brings every row back.
    setValue(filter, 'zzz');
    expect(visible()).toEqual([]);
    expect([...targets.querySelectorAll<HTMLElement>('fieldset.choices')].every((f) => f.hidden)).toBe(true);
    expect(text(targets.querySelector('.no-match'))).toBe('No modifier matches “zzz”.');
    setValue(filter, '');
    expect(visible()).toEqual(all);
    expect(targets.querySelector('.no-match')).toBeNull();

    // A paste into the filter is the field's, not a grab.
    const event = paste(filter, 'Item Class: Rings');
    expect(event.defaultPrevented).toBe(false);
    expect(bridge.calls).toEqual([]);

    // A new view keeps the filter text: it is the page's own.
    setValue(filter, 'cold');
    bridge.pushView(fixtureView('craft-fallback'));
    expect(labelled<HTMLInputElement>(root, 'Filter modifiers')!.value).toBe('cold');
    expect(visible().length).toBeGreaterThan(0);
    expect(visible().every((row) => row.toLowerCase().includes('cold'))).toBe(true);
    expect(bridge.calls).toEqual([]);
  });

  it('floor_choice_sends_set_targets', async () => {
    const { root, bridge } = await render(fixtureView('craft-planning'));
    const whole = (fire: number | null) => [
      { mod_id: 'suffix-to-fire-resistance', floor: fire },
      { mod_id: 'suffix-to-cold-resistance', floor: 3 },
      { mod_id: 'suffix-to-lightning-resistance', floor: 3 },
    ];

    // A floor from the chosen list: the whole list, that floor changed;
    // "as high as possible" sends no floor; the same select beside the
    // checked choice does the same.
    const chosen = root.querySelector('.targets .chosen')!;
    const group = root.querySelector('fieldset.choices[data-group="suffixes"]')!;
    choose(labelled<HTMLSelectElement>(chosen, 'Floor for +#% to Fire Resistance')!, '4');
    choose(labelled<HTMLSelectElement>(chosen, 'Floor for +#% to Fire Resistance')!, '');
    choose(labelled<HTMLSelectElement>(group, 'Floor for +#% to Fire Resistance')!, '2');
    expect(bridge.calls).toEqual([
      { name: 'set_targets', args: { targets: whole(4) } },
      { name: 'set_targets', args: { targets: whole(null) } },
      { name: 'set_targets', args: { targets: whole(2) } },
    ]);
    // The page shows the view's floor until the next view says otherwise.
    expect(labelled<HTMLSelectElement>(chosen, 'Floor for +#% to Fire Resistance')!.selectedOptions[0].textContent).toBe('At least T3');

    // Checking a choice adds it with the default floor; unchecking removes
    // it; Remove and Clear targets. Each sends the whole list.
    bridge.calls.length = 0;
    const regen = checkbox(root, '# Life Regeneration per second')!;
    expect(regen.checked).toBe(false);
    expect(labelled(group, 'Floor for # Life Regeneration per second')).toBeNull();
    click(regen);
    expect(regen.checked).toBe(false);
    click(checkbox(root, '+#% to Cold Resistance'));
    click(labelled(root, 'Remove +#% to Lightning Resistance'));
    click(button(root, 'Clear targets'));
    expect(bridge.calls).toEqual([
      { name: 'set_targets', args: { targets: [...whole(3), { mod_id: 'suffix-life-regeneration-per-second', floor: null }] } },
      { name: 'set_targets', args: { targets: [whole(3)[0], whole(3)[2]] } },
      { name: 'set_targets', args: { targets: whole(3).slice(0, 2) } },
      { name: 'set_targets', args: { targets: [] } },
    ]);

    // The next view is what the page shows.
    const lowered = fixtureView('craft-planning');
    lowered.craft.planning!.targets[0] = { ...lowered.craft.planning!.targets[0], floor: null, floor_in_force: 2 };
    bridge.pushView(lowered);
    expect(labelled<HTMLSelectElement>(chosen, 'Floor for +#% to Fire Resistance')!.selectedOptions[0].textContent).toBe(
      'As high as possible (T2)',
    );
  });

  it('craft_blocked_change_sends_set_craft_blocked', async () => {
    const { root, bridge } = await render(fixtureView('craft-fallback'));
    const blocked = root.querySelector<HTMLDetailsElement>('details.blocked')!;

    // The picker lists the catalogue by name in its groups; there is no id field.
    expect(texts(blocked, 'fieldset.picker-group > legend')).toEqual(CATALOGUE.map((g) => g.label));
    expect(blocked.querySelectorAll('input[type="checkbox"][data-id]')).toHaveLength(
      CATALOGUE.reduce((n, g) => n + g.entries.length, 0),
    );
    expect(blocked.querySelectorAll('input[type="text"], input:not([type]), textarea')).toHaveLength(0);

    // Blocking one more, then unblocking this craft's own: the whole effective list each time.
    click(checkbox(blocked, 'Chaos Orb'));
    click(checkbox(blocked, 'Omen of Whittling'));
    expect(bridge.calls).toEqual([
      { name: 'set_craft_blocked', args: { blocked: ['omen-of-whittling', 'chaos-orb'] } },
      { name: 'set_craft_blocked', args: { blocked: [] } },
    ]);
    // The page shows the view's list until the next view says otherwise.
    expect(checkbox(blocked, 'Chaos Orb')!.checked).toBe(false);
    expect(checkbox(blocked, 'Omen of Whittling')!.checked).toBe(true);

    // A Settings entry is tagged so; one allowed for this craft is listed
    // unchecked with its tag, and checking it blocks it again.
    const mixed = fixtureView('craft-fallback');
    mixed.craft.planning!.blocked = {
      blocked: [
        { id: 'divine-orb', name: 'Divine Orb', from: 'settings' },
        { id: 'omen-of-whittling', name: 'Omen of Whittling', from: 'craft' },
      ],
      allowed: [{ id: 'exalted-orb', name: 'Exalted Orb' }],
    };
    bridge.calls.length = 0;
    bridge.pushView(mixed);
    expect(text(blocked.querySelector('summary'))).toBe('Blocked for this craft: 2');
    const divine = checkbox(blocked, 'Divine Orb')!;
    expect(divine.checked).toBe(true);
    expect(text(divine.closest('label')!.querySelector('.tag'))).toBe('Settings');
    const exalted = checkbox(blocked, 'Exalted Orb')!;
    expect(exalted.checked).toBe(false);
    expect(text(exalted.closest('label')!.querySelector('.tag'))).toBe('allowed for this craft');
    expect(checkbox(blocked, 'Chaos Orb')!.closest('label')!.querySelector('.tag')).toBeNull();
    click(exalted);
    expect(bridge.calls).toEqual([
      { name: 'set_craft_blocked', args: { blocked: ['divine-orb', 'omen-of-whittling', 'exalted-orb'] } },
    ]);

    // The picker's own filters send nothing.
    bridge.calls.length = 0;
    const shownNames = () =>
      [...blocked.querySelectorAll<HTMLElement>('li.entry')]
        .filter((li) => !li.hidden && !li.closest('fieldset')!.hidden)
        .map((li) => text(li.querySelector('.name')));
    setValue(labelled<HTMLInputElement>(blocked, 'Filter currencies')!, 'orb');
    expect(shownNames().length).toBeGreaterThan(3);
    for (const name of shownNames()) expect(name.toLowerCase()).toContain('orb');
    click(blocked.querySelector('.blocked-only input'));
    expect(shownNames()).toEqual(['Divine Orb']);
    setValue(labelled<HTMLInputElement>(blocked, 'Filter currencies')!, 'qqq');
    expect(shownNames()).toEqual([]);
    expect(text(blocked.querySelector('.no-match'))).toBe('No currency matches “qqq”.');
    expect(bridge.calls).toEqual([]);

    // The player's own closing holds while the mode stays planning.
    blocked.open = false;
    blocked.dispatchEvent(new Event('toggle'));
    await settle();
    bridge.pushView(fixtureView('craft-fallback'));
    expect(root.querySelector<HTMLDetailsElement>('details.blocked')!.open).toBe(false);
  });

  it('accept_sends_accept_craft', async () => {
    const { root, bridge } = await render(fixtureView('craft-planning'));
    click(button(root, 'Start crafting'));
    expect(bridge.calls).toEqual([{ name: 'accept_craft', args: {} }]);

    // Another applicable route is chosen through the route select.
    bridge.calls.length = 0;
    choose(labelled<HTMLSelectElement>(root.querySelector('.route')!, 'Route')!, 'rings-flat-attack');
    expect(bridge.calls).toEqual([{ name: 'choose_route', args: { blueprint: 'rings-flat-attack' } }]);

    // Negative control: a disabled Start sends nothing, and with no other
    // route there is no route select.
    const noRoute = await render(fixtureView('craft-no-route'));
    click(button(noRoute.root, 'Start crafting'));
    expect(noRoute.bridge.calls).toEqual([]);
    expect(labelled(noRoute.root, 'Route')).toBeNull();

    // The view that starts the guide moves focus to the new mode's first heading.
    bridge.pushView(fixtureView('craft-guiding'));
    await settle();
    expect(document.activeElement).toBe(root.querySelector('.craft [data-mode-heading]'));
    expect(text(document.activeElement)).toBe('Doom Loop');
  });

  it('guide_shows_branches_and_history', async () => {
    const guidingView = fixtureView('craft-guiding');
    const g = guidingView.craft.guiding!;
    const { root, bridge } = await render(guidingView);
    const guide = root.querySelector('.guide')!;
    expect(text(guide.querySelector('.guide-status'))).toBe('Crafting');
    expect(text(guide.querySelector('.last'))).toBe(`Last: Hit from a grab ${g.history[0].summary}`);
    const card = guide.querySelector('.card')!;
    expect(text(card.querySelector('.step-no'))).toBe('Step 2');
    expect(text(card.querySelector('.try'))).toBe('try 1');
    expect(text(card.querySelector('.instruction'))).toBe(g.step!.step.instruction);
    expect(text(card.querySelector('.uses'))).toBe('Uses Chaos Orb, Omen of Whittling');
    expect(text(card.querySelector('.odds'))).toBe(g.step!.step.odds);
    expect(texts(card, '.branch')).toEqual([g.step!.on_hit, g.step!.on_miss]);
    expect(text(card.querySelector('.prompt'))).toBe(
      `Take this step in the game, copy the item with Ctrl+Alt+C, then press ${guidingView.hotkey} or Ctrl+V here.`,
    );
    const history = root.querySelector('.history')!;
    expect(text(history.querySelector('h2'))).toBe('History');
    expect(texts(history, 'ol > li')).toEqual([`Step 1 ${g.history[0].instruction} Hit from a grab ${g.history[0].summary}`]);
    expect(root.querySelector('.item-now .item-panel')).not.toBeNull();

    // End craft asks first while a craft is in progress; Keep crafting
    // closes the question and gives the focus back.
    const end = button(root.querySelector('.end-bar')!, 'End craft')!;
    expect(end.classList.contains('primary')).toBe(false);
    click(end);
    await settle();
    const question = root.querySelector('.end-bar [role="group"]')!;
    expect(text(question.querySelector(`#${question.getAttribute('aria-labelledby')}`))).toBe('End this craft? Its history is cleared.');
    expect(document.activeElement).toBe(button(question, 'Keep crafting'));
    expect(bridge.calls).toEqual([]);
    click(button(question, 'Keep crafting'));
    await settle();
    expect(root.querySelector('.end-bar [role="group"]')).toBeNull();
    expect(document.activeElement).toBe(button(root.querySelector('.end-bar')!, 'End craft'));
    click(button(root.querySelector('.end-bar')!, 'End craft'));
    click(button(root.querySelector('.end-bar [role="group"]')!, 'End craft'));
    expect(bridge.calls).toEqual([{ name: 'end_craft', args: {} }]);

    // A miss: the Last line, the step on its second try, the Miss entry.
    const missView = fixtureView('craft-miss');
    const m = missView.craft.guiding!;
    const miss = await render(missView);
    expect(text(miss.root.querySelector('.last'))).toBe(`Last: Miss from a grab ${m.history[0].summary}`);
    expect(miss.root.querySelector('.last .result')!.getAttribute('data-result')).toBe('miss');
    expect(text(miss.root.querySelector('.card .try'))).toBe('try 2');
    expect(texts(miss.root, '.card .branch')).toEqual([m.step!.on_hit, 'On a miss: this step again']);
    expect(miss.root.querySelector('.history li .result')!.getAttribute('data-result')).toBe('miss');

    // Replan: the reasons under "Why:", the hint, End craft primary and at once.
    const replanView = fixtureView('craft-replan');
    const replan = await render(replanView);
    expect(text(replan.root.querySelector('.guide-status'))).toBe('Replan');
    expect(replan.root.querySelector('.card')).toBeNull();
    expect(text(replan.root.querySelector('.why-title'))).toBe('Why:');
    expect(texts(replan.root, '.why-list li')).toEqual(replanView.craft.guiding!.reasons);
    expect(text(replan.root.querySelector('.replan-hint'))).toBe('End the craft to plan again from this item.');
    const replanEnd = button(replan.root.querySelector('.end-bar')!, 'End craft')!;
    expect(replanEnd.classList.contains('primary')).toBe(true);
    click(replanEnd);
    expect(replan.bridge.calls).toEqual([{ name: 'end_craft', args: {} }]);

    // Done: the card replaced, the whole history in order.
    const doneView = fixtureView('craft-done');
    const d = doneView.craft.guiding!;
    const done = await render(doneView);
    expect(text(done.root.querySelector('.guide-status'))).toBe('Done');
    expect(texts(done.root, '.done p')).toEqual(['The craft is done.', d.history[d.history.length - 1].summary]);
    expect(done.root.querySelector('.card')).toBeNull();
    expect(texts(done.root, '.history .summary')).toEqual(d.history.map((h) => h.summary));
    expect(texts(done.root, '.history .step-no')).toEqual(['Step 1', 'Step 2', 'Step 3', 'Step 4']);
    expect(button(done.root.querySelector('.end-bar')!, 'End craft')!.classList.contains('primary')).toBe(true);

    // No history yet, and a stuck guide's reasons with its card.
    const fresh = fixtureView('craft-guiding');
    fresh.craft.guiding = { ...fresh.craft.guiding!, status: 'stuck', reasons: ['no alternative of this step is usable'], history: [] };
    const stuck = await render(fresh);
    expect(text(stuck.root.querySelector('.history .empty'))).toBe('No step taken yet.');
    expect(stuck.root.querySelector('.last')).toBeNull();
    expect(text(stuck.root.querySelector('.guide-status'))).toBe('Stuck');
    expect(stuck.root.querySelector('.card')).not.toBeNull();
    expect(texts(stuck.root, '.why-list li')).toEqual(['no alternative of this step is usable']);
    expect(stuck.root.querySelector('.replan-hint')).toBeNull();
  });

  it('history_marks_outcomes_read_from_the_screen', async () => {
    // A hit the screen watch read: the Last line and History's entry name
    // the source in words, as a grab's do; the page maps the source alone.
    const hitView = fixtureView('craft-screen-hit');
    const h = hitView.craft.guiding!;
    expect(h.history.map((e) => e.source)).toEqual(['screen']);
    const hit = await render(hitView);
    expect(text(hit.root.querySelector('.last'))).toBe(`Last: Hit from the screen ${h.history[0].summary}`);
    expect(text(hit.root.querySelector('.last .source'))).toBe('from the screen');
    const entry = hit.root.querySelector('.history ol > li')!;
    expect(text(entry)).toBe(`Step 1 ${h.history[0].instruction} Hit from the screen ${h.history[0].summary}`);
    expect(text(entry.querySelector('.source'))).toBe('from the screen');
    // The item now is the read one, and the guide moved to the next step.
    expect(text(hit.root.querySelector('.card .step-no'))).toBe('Step 2');

    // A grab's entry says so; a history of both keeps each entry's own.
    const mixed = fixtureView('craft-done');
    const d = mixed.craft.guiding!;
    d.history = d.history.map((e, i) => ({ ...e, source: i % 2 === 0 ? 'grab' : 'screen' }));
    const done = await render(mixed);
    expect(texts(done.root, '.history .source')).toEqual(['from a grab', 'from the screen', 'from a grab', 'from the screen']);

    // An unexpected change read from the screen is not kept: no entry, no Last line.
    const unexpected = await render(fixtureView('craft-screen-unexpected'));
    expect(unexpected.root.querySelector('.last')).toBeNull();
    expect(text(unexpected.root.querySelector('.history .empty'))).toBe('No step taken yet.');
    expect(unexpected.bridge.calls).toEqual([]);
  });

  it('guiding_heading_does_not_name_the_blueprint', async () => {
    // The guide's heading is the item's title as the Item panel says it
    // (its name, else its base type); the status word beside it says
    // "Crafting". No blueprint name is drawn anywhere in the window.
    expect(GUIDING_FIXTURES).toContain('craft-miss-clean');
    for (const name of GUIDING_FIXTURES) {
      const g = fixtureView(name).craft.guiding!;
      expect(g.route_name, name).not.toBe('');
      const { root } = await renderPreview(name);
      const heading = root.querySelector('.guide [data-mode-heading]');
      expect(heading?.tagName, name).toBe('H2');
      expect(text(heading), name).toBe(g.item.name ?? g.item.base_type ?? g.item.class);
      expect(text(heading), name).toBe(text(root.querySelector('.item-now .item-title')));
      for (const blueprintName of BLUEPRINT_NAMES) expect(root.textContent, `${name}: ${blueprintName}`).not.toContain(blueprintName);
    }

    // An item with no name is titled by its base type.
    const unnamed = fixtureView('craft-guiding');
    unnamed.craft.guiding!.item.name = null;
    const { root } = await render(unnamed);
    expect(text(root.querySelector('.guide [data-mode-heading]'))).toBe(unnamed.craft.guiding!.item.base_type);
  });

  it('last_line_says_the_outcome_word_once', async () => {
    // The page says the outcome's word (bold, by its result) and Rust's
    // summary, which carries no word of its own: once in the Last line,
    // once in each History entry.
    let seen = 0;
    for (const name of GUIDING_FIXTURES) {
      const g = fixtureView(name).craft.guiding!;
      const { root } = await renderPreview(name);
      const entries = [...root.querySelectorAll('.history ol > li .outcome')];
      expect(entries, name).toHaveLength(g.history.length);
      g.history.forEach((entry, i) => {
        expect(entry.summary, name).not.toMatch(/^(hit|miss|unexpected)\b/i);
        expect(outcomeWords(text(entries[i])), `${name} #${i}`).toBe(1);
        expect(text(entries[i].querySelector('.result')), `${name} #${i}`).toBe(OUTCOME_LABELS[entry.result]);
        seen++;
      });
      const last = root.querySelector('.guide .last');
      if (g.history.length === 0 || g.status === 'done') {
        expect(last, name).toBeNull();
        continue;
      }
      const newest = g.history[g.history.length - 1];
      expect(outcomeWords(text(last)), name).toBe(1);
      expect(text(last!.querySelector('.result')), name).toBe(OUTCOME_LABELS[newest.result]);
    }
    expect(seen).toBeGreaterThan(3);

    // Control: a summary that starts with its word is said twice, and the count says so.
    const doubled = fixtureView('craft-miss');
    const entry = doubled.craft.guiding!.history[0];
    entry.summary = `Miss: ${entry.summary}`;
    const { root } = await render(doubled);
    expect(outcomeWords(text(root.querySelector('.guide .last')))).toBe(2);
    expect(outcomeWords(text(root.querySelector('.history .outcome')))).toBe(2);
  });

  it('guiding_shows_the_watch_line', async () => {
    // The card's Screen line is the view's watch line, as sent, in the
    // warning colour exactly when Rust marks it; the grab prompt stays in
    // every watch state, since the grab is the fallback each line points to.
    const marks: Record<string, string> = {};
    for (const name of [
      'craft-guiding',
      'craft-watching',
      'craft-screen-hit',
      'craft-screen-unexpected',
      'craft-watch-off',
      'craft-watch-unavailable',
      'overlay-active',
      'overlay-not-in-front',
    ]) {
      const view = fixtureView(name);
      const watch = view.craft.guiding!.watch!;
      const r = await render(view);
      const line = r.root.querySelector('.card .watch')!;
      expect(text(line.querySelector('.label')), name).toBe('Screen');
      expect(text(line.querySelector('.watch-text')), name).toBe(watch.text);
      marks[name] = line.getAttribute('data-warning')!;
      expect(text(r.root.querySelector('.card .prompt')), name).toMatch(/^Take this step in the game/);
    }
    expect(marks).toEqual({
      'craft-guiding': 'false',
      'craft-watching': 'false',
      'craft-screen-hit': 'false',
      'craft-screen-unexpected': 'true',
      'craft-watch-off': 'false',
      'craft-watch-unavailable': 'true',
      'overlay-active': 'false',
      'overlay-not-in-front': 'false',
    });

    // Stuck: the card and its watch line stay.
    const stuckView = fixtureView('craft-watching');
    stuckView.craft.guiding = {
      ...stuckView.craft.guiding!,
      status: 'stuck',
      reasons: ['no alternative of this step is usable'],
      watch: { text: 'Paused: the guide is stuck.', warning: false },
    };
    const stuck = await render(stuckView);
    expect(text(stuck.root.querySelector('.card .watch .watch-text'))).toBe('Paused: the guide is stuck.');

    // Replan and done: no card, no watch line (Rust sends none).
    for (const name of ['craft-replan', 'craft-done']) {
      const view = fixtureView(name);
      expect(view.craft.guiding!.watch, name).toBeNull();
      const r = await render(view);
      expect(r.root.querySelector('.watch'), name).toBeNull();
    }
  });

  it('unknown_odds_are_said_in_words', async () => {
    const view = fixtureView('craft-planning');
    const steps = view.craft.planning!.preview!.route!.steps;
    const { root } = await render(view);
    const shown = texts(root, 'ol.steps > li .odds');
    expect(shown).toEqual(steps.map((s) => s.odds));
    // Known odds and unknown ones, each as sent; never a blank or a dash.
    expect(shown.filter((o) => o.startsWith('~'))).toHaveLength(2);
    expect(shown.filter((o) => o.startsWith('odds unknown: '))).toHaveLength(2);
    for (const odds of shown) {
      expect(odds.trim()).not.toBe('');
      expect(odds).not.toMatch(/^[-—–]$/);
    }

    // A manual step (no odds, no currency) shows no odds or Uses line rather than an empty one.
    const manual = fixtureView('craft-planning');
    const route = manual.craft.planning!.preview!.route!;
    route.steps[3] = { ...route.steps[3], odds: null, uses: [] };
    const r = await render(manual);
    const last = r.root.querySelectorAll('ol.steps > li')[3];
    expect(last.querySelector('.odds')).toBeNull();
    expect(last.querySelector('.uses')).toBeNull();

    // The guide says unknown odds the same way.
    const guiding = fixtureView('craft-guiding');
    guiding.craft.guiding!.step!.step.odds = 'odds unknown: desecration and reveals have no established weights';
    const gr = await render(guiding);
    expect(text(gr.root.querySelector('.card .odds'))).toBe('odds unknown: desecration and reveals have no established weights');
  });

  it('craft_of_exile_link_sends_its_command_without_a_url', async () => {
    for (const name of ['empty', 'craft-planning', 'craft-guiding']) {
      const { root, bridge } = await render(fixtureView(name));
      const link = button(root, 'Explore more crafting on Craft of Exile')!;
      // A native button, said to open the web browser, holding no address.
      expect(link.tagName, name).toBe('BUTTON');
      expect(link.getAttribute('type'), name).toBe('button');
      expect(link.closest('a'), name).toBeNull();
      expect(text(root.querySelector(`#${link.getAttribute('aria-describedby')}`)), name).toBe(
        'Opens Craft of Exile in your web browser',
      );
      expect(link.querySelector('svg')?.getAttribute('aria-hidden'), name).toBe('true');
      expect(link.outerHTML, name).not.toMatch(/https?:|href|craftofexile/i);
      expect(link.classList.contains('primary'), name).toBe(false);

      const open = vi.spyOn(window, 'open').mockImplementation(() => null);
      const before = window.location.href;
      click(link);
      expect(bridge.calls, name).toEqual([{ name: 'open_craft_of_exile', args: {} }]);
      expect(open, name).not.toHaveBeenCalled();
      expect(window.location.href, name).toBe(before);
      open.mockRestore();
    }

    // Inside the app, the command goes out with no argument at all.
    const asked: Array<[string, unknown]> = [];
    mockIPC((cmd, payload) => {
      asked.push([cmd, payload]);
      return undefined;
    });
    await tauriBridge().command('open_craft_of_exile', {});
    expect(asked).toEqual([['open_craft_of_exile', {}]]);
  });

  it('no_page_renders_an_external_link', async () => {
    const outside = /^(?:[a-z][a-z0-9+.-]*:|\/\/)/i;
    const check = (root: Element, name: string) => {
      for (const el of root.querySelectorAll('[href], [src], [action], [formaction]')) {
        for (const attr of ['href', 'src', 'action', 'formaction']) {
          const value = el.getAttribute(attr);
          if (value !== null) expect(value, `${name}: ${el.tagName} ${attr}`).not.toMatch(outside);
        }
      }
      expect(root.querySelectorAll('a[href], [target], form, iframe').length, name).toBe(0);
      expect(root.innerHTML, name).not.toMatch(/https?:\/\//i);
    };
    const open = vi.spyOn(window, 'open').mockImplementation(() => null);
    const before = window.location.href;

    // The main window in every state, every disclosure opened, every button pressed.
    for (const name of FIXTURE_NAMES) {
      const r = await render(fixtureView(name));
      check(r.root, name);
      for (const details of r.root.querySelectorAll('details')) details.open = true;
      check(r.root, `${name} opened`);
      for (const b of r.root.querySelectorAll<HTMLButtonElement>('button:not(:disabled)')) click(b);
      check(r.root, `${name} pressed`);
    }
    // The overlay, over the states a craft goes through.
    for (const name of ['overlay-active', ...CRAFT_FIXTURE_NAMES]) {
      const r = await renderComponent(Overlay, { bridge: fakeBridge(fixtureView(name)) }, () => true);
      mounted.push(r);
      check(r.root, `overlay ${name}`);
    }
    expect(open).not.toHaveBeenCalled();
    expect(window.location.href).toBe(before);
  });
});
