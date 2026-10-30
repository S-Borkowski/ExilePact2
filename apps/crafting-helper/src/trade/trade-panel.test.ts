// The *Find a base on trade* panel (`ui-design.md` § "Find a base on
// trade"): every `trade-*` view fixture renders in the craft area's place,
// each control sends `set_trade_search` with the whole selection, a default
// weight is labelled with Rust's reason, the link is read-only text and the
// Search button sends its command with no address. The Settings window's
// Trade section is tested beside the rest of that window
// (`../settings/settings-window.test.ts`).

import { afterEach, describe, expect, it, vi } from 'vitest';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { tauriBridge } from '../bridge';
import type { TradeView, View } from '../ipc';
import {
  type Rendered,
  TRADE_FIXTURE_NAMES,
  button,
  choose,
  click,
  fixtureView,
  readBytes,
  renderView,
  setValue,
  text,
} from '../test-support';

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

/** The text of each element `selector` finds under `root`. */
const texts = (root: Element, selector: string): string[] => [...root.querySelectorAll(selector)].map((el) => text(el));
/** The control a visible `<label for>` reading `name` names. */
function field<E extends HTMLElement>(root: Element, name: string): E {
  const label = [...root.querySelectorAll<HTMLLabelElement>('label[for]')].find((l) => text(l) === name);
  if (!label) throw new Error(`no label "${name}"`);
  return document.getElementById(label.htmlFor) as E;
}
/** A modifier row's checkbox, by the modifier's text. */
const modBox = (root: Element, modText: string): HTMLInputElement | undefined =>
  [...root.querySelectorAll<HTMLInputElement>('.mod-group input[type="checkbox"]')].find(
    (box) => text(box.closest('label')?.querySelector('.name')) === modText,
  );
/** The radio whose choice reads `name`. */
const radio = (root: Element, name: string): HTMLInputElement =>
  [...root.querySelectorAll<HTMLInputElement>('input[type="radio"]')].find(
    (r) => text(r.closest('label')?.querySelector('.choice-name')) === name,
  )!;
/** Lets a focus move that waits for a tick happen. */
const settle = () => new Promise((resolve) => setTimeout(resolve, 0));

/** The trade part of a fixture, with the whole selection the page must send back. */
const tradeOf = (name: string): TradeView => fixtureView(name).trade;

describe('the trade panel', () => {
  it('trade_panel_renders_every_trade_fixture', async () => {
    for (const name of TRADE_FIXTURE_NAMES) {
      const view = fixtureView(name);
      const t = view.trade;
      const sel = t.selection;
      expect(sel.open, name).toBe(true);
      const { root, bridge } = await render(view);

      // The panel in the craft area's place, the craft untouched underneath.
      const panel = root.querySelector('.trade')!;
      expect(panel, name).not.toBeNull();
      expect(root.querySelector('.craft'), name).toBeNull();
      expect(root.querySelector('.app')!.getAttribute('data-mode'), name).toBe(view.craft.mode);
      expect(text(panel.querySelector('h2')), name).toBe('Find a base on trade');
      const head = panel.querySelector('.head')!;
      expect(text(head.querySelector('button')), name).toBe('Back to the craft');
      expect(head.querySelector('button svg')?.getAttribute('aria-hidden'), name).toBe('true');
      const find = button(root, 'Find a base on trade')!;
      expect(find.getAttribute('aria-pressed'), name).toBe('true');

      // Base: the class and base selects with Rust's names, the level field.
      const cls = field<HTMLSelectElement>(panel, 'Item class');
      expect([...cls.options].map((o) => o.textContent), name).toEqual(['Choose a class', ...t.classes]);
      expect(cls.value, name).toBe(sel.class ?? '');
      const base = field<HTMLSelectElement>(panel, 'Base type');
      expect(base.disabled, name).toBe(sel.class === null);
      expect([...base.options].map((o) => o.textContent), name).toEqual(['Choose a base', ...t.bases]);
      expect(base.value, name).toBe(sel.base ?? '');
      const level = field<HTMLInputElement>(panel, 'Minimum item level');
      expect(level.value, name).toBe(sel.item_level === null ? '' : String(sel.item_level));
      expect(text(document.getElementById(level.getAttribute('aria-describedby')!)), name).toBe(
        'Optional. Empty searches any item level; a higher level reaches better tiers.',
      );

      // Modifiers: a prompt before a base, else each side's rows with their weights.
      const mods = panel.querySelector('.modifiers')!;
      expect(text(mods.querySelector('h3')), name).toBe('Modifiers');
      if (sel.class === null) {
        expect(text(mods.querySelector('.prompt')), name).toBe('Choose an item class to see its modifiers.');
      } else if (sel.base === null) {
        expect(text(mods.querySelector('.prompt')), name).toBe('Choose a base type to see its modifiers.');
      } else {
        expect(mods.querySelector('.prompt'), name).toBeNull();
        expect(text(mods.querySelector('.help')), name).toBe(
          'Weight: higher is rarer. 12.0 means about 1 in 12 modifiers added to that side of this base is this one, at the floor or better. The search ranks results by these weights.',
        );
        for (const [key, label, list] of [
          ['prefixes', 'Prefixes', t.prefixes],
          ['suffixes', 'Suffixes', t.suffixes],
        ] as const) {
          const group = mods.querySelector(`fieldset[data-group="${key}"]`)!;
          expect(text(group.querySelector('legend')), `${name} ${key}`).toBe(
            `${label} · ${list.filter((m) => m.picked).length} picked`,
          );
          expect(texts(group, 'li .name'), `${name} ${key}`).toEqual(list.map((m) => m.text));
          expect(texts(group, 'li .weight'), `${name} ${key}`).toEqual(list.map((m) => m.weight));
          expect(
            [...group.querySelectorAll<HTMLInputElement>('input[type="checkbox"]')].map((b) => b.checked),
            `${name} ${key}`,
          ).toEqual(list.map((m) => m.picked));
          // A picked row's floor select carries Rust's labels, its floor selected.
          for (const m of list.filter((m) => m.picked)) {
            const floor = mods.querySelector<HTMLSelectElement>(`[aria-label="Floor for ${m.text}"]`)!;
            expect([...floor.options].map((o) => o.textContent), m.mod_id).toEqual(m.floors.map((f) => f.label));
            expect(floor.selectedOptions[0].textContent, m.mod_id).toBe(m.floors.find((f) => f.floor === m.floor)!.label);
          }
          expect(group.querySelectorAll('select').length, `${name} ${key}`).toBe(list.filter((m) => m.picked).length);
        }
        // Rows that cannot be searched sit in one closed disclosure, with no checkbox.
        const unsearchable = mods.querySelector<HTMLDetailsElement>('details.not-searchable');
        if (t.not_searchable.length === 0) {
          expect(unsearchable, name).toBeNull();
        } else {
          expect(unsearchable!.open, name).toBe(false);
          expect(text(unsearchable!.querySelector('summary')), name).toBe(`Not searchable on trade: ${t.not_searchable.length}`);
          expect(unsearchable!.querySelector('input'), name).toBeNull();
          expect([...unsearchable!.querySelectorAll('li')].map((li) => texts(li, 'span')), name).toEqual(
            t.not_searchable.map((m) => [m.text, m.side, m.reason]),
          );
        }
      }

      // Match: Both slots or At least one, as the view says, and Rust's count line.
      expect(radio(panel, 'Both slots').checked, name).toBe(sel.both_slots);
      expect(radio(panel, 'At least one').checked, name).toBe(!sel.both_slots);
      expect(texts(panel, '.match .hint'), name).toEqual(['A prefix and a suffix from your picks.', 'Any one of your picks.']);
      const count = panel.querySelector('.count-line');
      if (t.count_line === null) expect(count, name).toBeNull();
      else expect(text(count), name).toBe(t.count_line);

      // League, and Change, which opens Settings.
      expect(text(panel.querySelector('.league-label')), name).toBe('League');
      expect(text(panel.querySelector('.league-name')), name).toBe(t.league);
      expect(button(panel, 'Change'), name).toBeDefined();

      // The link, only when the view carries one.
      const link = panel.querySelector<HTMLTextAreaElement>('textarea');
      if (t.link === null) expect(link, name).toBeNull();
      else expect(link!.value, name).toBe(t.link);

      // The Search bar: enabled as the view says, why beside it when not.
      const search = button(panel, 'Search on the trade site')!;
      expect(search.disabled, name).toBe(!t.can_search);
      const why = panel.querySelector('.search-bar .why');
      if (t.why_not === null) expect(why, name).toBeNull();
      else expect(text(why), name).toBe(`${t.why_not.kind === 'error' ? '!' : ''}${t.why_not.text}`);

      // The status line is the app's, as in any mode.
      const status = root.querySelector('[role="status"]')!;
      expect(text(status), name).toBe(`${view.status!.kind === 'error' ? '!' : ''}${view.status!.text}`);
      expect(bridge.calls, name).toEqual([]);
    }

    // Each state's own words.
    const empty = await render(fixtureView('trade-empty'));
    expect(text(empty.root.querySelector('.search-bar .why'))).toBe('Choose an item class.');
    expect(empty.root.querySelector('.count-line')).toBeNull();
    const refused = await render(fixtureView('trade-refused'));
    expect(text(refused.root.querySelector('[role="status"]'))).toBe('!Trade search not changed: Rings has no base type Iron Mitts.');
    // A refusal keeps the selection and the link as they were.
    expect(tradeOf('trade-refused').selection).toEqual(tradeOf('trade-picking').selection);
    expect(refused.root.querySelector('textarea')!.value).toBe(tradeOf('trade-picking').link);

    // No base: the base select on "Choose a base", the prompt, Rust's reason.
    const noBase = fixtureView('trade-empty');
    noBase.trade.selection.class = 'Rings';
    noBase.trade.bases = tradeOf('trade-picking').bases;
    noBase.trade.why_not = { kind: 'info', text: 'Choose a base type.' };
    const nb = await render(noBase);
    expect(field<HTMLSelectElement>(nb.root, 'Base type').disabled).toBe(false);
    expect(field<HTMLSelectElement>(nb.root, 'Base type').value).toBe('');
    expect(text(nb.root.querySelector('.modifiers .prompt'))).toBe('Choose a base type to see its modifiers.');
    expect(text(nb.root.querySelector('.search-bar .why'))).toBe('Choose a base type.');

    // A link Rust cannot make: no link, its reason marked "!", in the warning colour.
    const tooLong = fixtureView('trade-picking');
    tooLong.trade = { ...tooLong.trade, link: null, can_search: false, why_not: { kind: 'error', text: 'Too many modifiers for one link: remove one.' } };
    const tl = await render(tooLong);
    expect(tl.root.querySelector('textarea')).toBeNull();
    const why = tl.root.querySelector('.search-bar .why')!;
    expect(why.getAttribute('data-kind')).toBe('error');
    expect(text(why)).toBe('!Too many modifiers for one link: remove one.');
    expect(why.querySelector('.mark')?.getAttribute('aria-hidden')).toBe('true');

    // Eight picks: Rust's limit line, the unpicked rows disabled, the picked ones not.
    const full = fixtureView('trade-picking');
    full.trade = { ...full.trade, can_pick: false, pick_limit: 'At most 8 modifiers in one search.' };
    const f = await render(full);
    expect(text(f.root.querySelector('.modifiers .limit'))).toBe('At most 8 modifiers in one search.');
    const boxes = [...f.root.querySelectorAll<HTMLInputElement>('.mod-group input[type="checkbox"]')];
    expect(boxes.filter((b) => b.disabled).length).toBe(boxes.filter((b) => !b.checked).length);
    expect(boxes.filter((b) => b.checked).every((b) => !b.disabled)).toBe(true);
    expect(fixtureView('trade-picking').trade.pick_limit).toBeNull();

    // Closed (every other fixture): the craft area, the button not pressed, no panel.
    for (const name of ['empty', 'craft-planning', 'craft-guiding']) {
      const r = await render(fixtureView(name));
      expect(r.root.querySelector('.trade'), name).toBeNull();
      expect(r.root.querySelector('.craft'), name).not.toBeNull();
      expect(button(r.root, 'Find a base on trade')!.getAttribute('aria-pressed'), name).toBe('false');
    }
  });

  it('trade_pick_sends_set_trade_search', async () => {
    // The header's button opens the panel with the selection as the view holds it.
    const planning = fixtureView('craft-planning');
    const closed = planning.trade.selection;
    const { root, bridge } = await render(planning);
    const find = button(root, 'Find a base on trade')!;
    expect(find.querySelector('svg')).toBeNull();
    click(find);
    expect(bridge.calls).toEqual([{ name: 'set_trade_search', args: { search: { ...closed, open: true } } }]);
    // Nothing changes until the view says so; then the panel, focus on its heading.
    expect(root.querySelector('.trade')).toBeNull();
    bridge.pushView(fixtureView('trade-empty'));
    await settle();
    expect(document.activeElement).toBe(root.querySelector('.trade h2'));

    // A class: the whole selection with the class; Rust clears the rest.
    const empty = tradeOf('trade-empty').selection;
    const cls = field<HTMLSelectElement>(root, 'Item class');
    choose(cls, 'Rings');
    expect(bridge.calls[1]).toEqual({ name: 'set_trade_search', args: { search: { ...empty, class: 'Rings' } } });
    expect(cls.value).toBe('');

    // On a base with picks: each control sends the whole selection, one field changed.
    bridge.pushView(fixtureView('trade-picking'));
    const sel = tradeOf('trade-picking').selection;
    const t = tradeOf('trade-picking');
    bridge.calls.length = 0;

    const unpicked = t.prefixes.find((m) => !m.picked)!;
    const box = modBox(root, unpicked.text)!;
    click(box);
    expect(bridge.calls.at(-1)).toEqual({
      name: 'set_trade_search',
      args: { search: { ...sel, picks: [...sel.picks, { mod_id: unpicked.mod_id, floor: null }] } },
    });
    // The box shows the view's state until the next view says otherwise.
    expect(box.checked).toBe(false);

    const life = t.prefixes.find((m) => m.mod_id === 'prefix-to-maximum-life')!;
    expect(life.picked).toBe(true);
    click(modBox(root, life.text));
    expect(bridge.calls.at(-1)).toEqual({
      name: 'set_trade_search',
      args: { search: { ...sel, picks: sel.picks.filter((p) => p.mod_id !== life.mod_id) } },
    });
    expect(modBox(root, life.text)!.checked).toBe(true);

    const floor = root.querySelector<HTMLSelectElement>(`[aria-label="Floor for ${life.text}"]`)!;
    choose(floor, '1');
    expect(bridge.calls.at(-1)).toEqual({
      name: 'set_trade_search',
      args: { search: { ...sel, picks: sel.picks.map((p) => (p.mod_id === life.mod_id ? { mod_id: p.mod_id, floor: 1 } : p)) } },
    });
    expect(floor.value).toBe('');

    const base = field<HTMLSelectElement>(root, 'Base type');
    const other = t.bases.find((b) => b !== sel.base)!;
    choose(base, other);
    expect(bridge.calls.at(-1)).toEqual({ name: 'set_trade_search', args: { search: { ...sel, base: other } } });
    expect(base.value).toBe(sel.base);

    // The item level: a whole number, or empty for none; anything else is not sent.
    const level = field<HTMLInputElement>(root, 'Minimum item level');
    setValue(level, '82');
    expect(bridge.calls.at(-1)).toEqual({ name: 'set_trade_search', args: { search: { ...sel, item_level: 82 } } });
    expect(level.value).toBe('75');
    setValue(level, '');
    expect(bridge.calls.at(-1)).toEqual({ name: 'set_trade_search', args: { search: { ...sel, item_level: null } } });
    const sent = bridge.calls.length;
    setValue(level, '2.5');
    expect(bridge.calls).toHaveLength(sent);
    expect(level.value).toBe('75');
    // Out of range goes to Rust, which refuses it in words.
    setValue(level, '101');
    expect(bridge.calls.at(-1)).toEqual({ name: 'set_trade_search', args: { search: { ...sel, item_level: 101 } } });
    // Rust's refusal is a view with the selection as it was.
    bridge.pushView(fixtureView('trade-picking'));

    // Match.
    click(radio(root, 'At least one'));
    expect(bridge.calls.at(-1)).toEqual({ name: 'set_trade_search', args: { search: { ...sel, both_slots: false } } });
    expect(radio(root, 'Both slots').checked).toBe(true);

    // The filter is the page's own: it narrows the rows and sends nothing.
    const before = bridge.calls.length;
    const filter = root.querySelector<HTMLInputElement>('input[aria-label="Filter modifiers"]')!;
    setValue(filter, 'resistance');
    const shown = [...root.querySelectorAll<HTMLLIElement>('.mod-group li')].filter((li) => !li.hidden).map((li) => text(li.querySelector('.name')));
    expect(shown.length).toBeGreaterThan(0);
    expect(shown.every((s) => s.toLowerCase().includes('resistance'))).toBe(true);
    expect(root.querySelector<HTMLElement>('fieldset[data-group="prefixes"]')!.hidden).toBe(
      !t.prefixes.some((m) => m.text.toLowerCase().includes('resistance')),
    );
    setValue(filter, 'zzz');
    expect(text(root.querySelector('.no-match'))).toBe('No modifier matches “zzz”.');
    expect(bridge.calls).toHaveLength(before);

    // Change opens Settings, where the league is set.
    click(button(root, 'Change'));
    expect(bridge.calls.at(-1)).toEqual({ name: 'open_settings', args: {} });

    // Back to the craft, and the header's button while open, close the panel.
    click(button(root, 'Back to the craft'));
    expect(bridge.calls.at(-1)).toEqual({ name: 'set_trade_search', args: { search: { ...sel, open: false } } });
    click(button(root, 'Find a base on trade'));
    expect(bridge.calls.at(-1)).toEqual({ name: 'set_trade_search', args: { search: { ...sel, open: false } } });
    // Closed: the craft area back as it stands, focus on the header's button.
    bridge.pushView(fixtureView('craft-planning'));
    await settle();
    expect(root.querySelector('.trade')).toBeNull();
    expect(root.querySelector('.craft')).not.toBeNull();
    expect(document.activeElement).toBe(button(root, 'Find a base on trade'));
  });

  it('trade_item_level_typed_before_a_pick_is_kept', async () => {
    // Live 2: a level typed before picking was lost once a modifier was
    // picked, because the pick was sent with the view's level before Rust's
    // answer to the level had come back.
    const view = fixtureView('trade-picking');
    view.trade.selection.item_level = null;
    const sel = view.trade.selection;
    const unpicked = view.trade.prefixes.find((m) => !m.picked)!;
    const withPick = [...sel.picks, { mod_id: unpicked.mod_id, floor: null }];

    // Committed (the field left): the pick sent before the next view carries it.
    {
      const { root, bridge } = await render(view);
      const level = field<HTMLInputElement>(root, 'Minimum item level');
      setValue(level, '75');
      expect(bridge.calls.at(-1)).toEqual({ name: 'set_trade_search', args: { search: { ...sel, item_level: 75 } } });
      click(modBox(root, unpicked.text));
      expect(bridge.calls.at(-1)).toEqual({
        name: 'set_trade_search',
        args: { search: { ...sel, item_level: 75, picks: withPick } },
      });
      // Once a view has answered, the view's level stands again.
      bridge.pushView(fixtureView('trade-picking'));
      bridge.calls.length = 0;
      click(radio(root, 'At least one'));
      expect(bridge.calls.at(-1)).toEqual({
        name: 'set_trade_search',
        args: { search: { ...tradeOf('trade-picking').selection, both_slots: false } },
      });
    }

    // Typed and not yet committed: a pick carries it, and a view does not wipe it.
    {
      const { root, bridge } = await render(view);
      const level = field<HTMLInputElement>(root, 'Minimum item level');
      level.value = '75';
      level.dispatchEvent(new Event('input', { bubbles: true }));
      bridge.pushView(view);
      expect(level.value).toBe('75');
      click(modBox(root, unpicked.text));
      expect(bridge.calls.at(-1)).toEqual({
        name: 'set_trade_search',
        args: { search: { ...sel, item_level: 75, picks: withPick } },
      });
      // Text that is not a whole number is not sent: the view's level goes.
      level.value = '2.5';
      level.dispatchEvent(new Event('input', { bubbles: true }));
      click(radio(root, 'At least one'));
      expect(bridge.calls.at(-1)).toEqual({ name: 'set_trade_search', args: { search: { ...sel, both_slots: false } } });
    }
  });

  it('trade_default_weight_is_labelled_with_its_reason', async () => {
    const view = fixtureView('trade-default-weight');
    const t = view.trade;
    const all = [...t.prefixes, ...t.suffixes];
    expect(all.every((m) => m.weight_source === 'default' && m.default_reason !== null)).toBe(true);
    const { root } = await render(view);
    const mods = root.querySelector('.modifiers')!;

    // The word explained once, above the groups.
    expect(text(mods.querySelector('.legend'))).toBe('“default” marks a weight the data does not give; a picked modifier says why.');
    for (const m of all) {
      const row = mods.querySelector(`li[data-mod="${m.mod_id}"]`)!;
      // The tag in words, inside the label, after the number; a screen reader
      // hears "<text>, weight 1.0, default".
      const label = row.querySelector('label')!;
      expect(text(label.querySelector('.tag')), m.mod_id).toBe('default');
      expect(text(label), m.mod_id).toBe(`${m.text}, weight ${m.weight}, default`);
      // A picked row says why, as Rust sends it; an unpicked one does not.
      const reason = row.querySelector('.picked .reason');
      if (m.picked) expect(text(reason), m.mod_id).toBe(`default: ${m.default_reason}`);
      else expect(reason, m.mod_id).toBeNull();
    }
    expect(mods.querySelectorAll('.picked .reason')).toHaveLength(all.filter((m) => m.picked).length);

    // A ring mixes the two: only the rows whose weight is the default carry
    // the tag; a spawn weight is the number alone.
    const ring = tradeOf('trade-picking');
    const ringMods = [...ring.prefixes, ...ring.suffixes];
    const defaults = ringMods.filter((m) => m.weight_source === 'default');
    expect(defaults.length).toBeGreaterThan(0);
    expect(defaults.length).toBeLessThan(ringMods.length);
    const picking = await render(fixtureView('trade-picking'));
    const mixed = picking.root.querySelector('.modifiers')!;
    expect(mixed.querySelector('.legend')).not.toBeNull();
    expect([...mixed.querySelectorAll('.mod-group li')].filter((li) => li.querySelector('.tag')).map((li) => li.getAttribute('data-mod'))).toEqual(
      defaults.map((m) => m.mod_id),
    );
    const life = ring.prefixes.find((m) => m.mod_id === 'prefix-to-maximum-life')!;
    expect(life.weight_source).toBe('spawn');
    expect(text(mixed.querySelector(`li[data-mod="${life.mod_id}"] label`))).toBe(`${life.text}, weight ${life.weight}`);
    expect(mixed.querySelectorAll('.picked .reason')).toHaveLength(0);

    // Spawn weights alone: no tag, no legend, no reason.
    const spawnOnly = fixtureView('trade-picking');
    spawnOnly.trade.prefixes = spawnOnly.trade.prefixes.filter((m) => m.weight_source === 'spawn');
    spawnOnly.trade.suffixes = spawnOnly.trade.suffixes.filter((m) => m.weight_source === 'spawn');
    const spawn = (await render(spawnOnly)).root.querySelector('.modifiers')!;
    expect(spawn.querySelector('.legend')).toBeNull();
    expect(spawn.querySelectorAll('.mod-group .tag')).toHaveLength(0);
  });

  it('trade_link_is_shown_read_only', async () => {
    const view = fixtureView('trade-picking');
    const link = view.trade.link!;
    expect(link.startsWith('https://www.pathofexile.com/trade2/search/poe2/')).toBe(true);
    const { root } = await render(view);

    // A read-only multi-line field, labelled, focusable and selectable, never a link.
    const area = field<HTMLTextAreaElement>(root, 'Trade link');
    expect(area.tagName).toBe('TEXTAREA');
    expect(area.readOnly).toBe(true);
    expect(area.disabled).toBe(false);
    expect(area.getAttribute('rows')).toBe('3');
    expect(area.getAttribute('tabindex')).toBeNull();
    expect(area.value).toBe(link);
    expect(text(document.getElementById(area.getAttribute('aria-describedby')!))).toBe('Select it and press Ctrl+C to copy it.');
    area.focus();
    expect(document.activeElement).toBe(area);
    area.select();
    expect(area.value.slice(area.selectionStart!, area.selectionEnd!)).toBe(link);

    // No anchor, no address in the markup: the link is the field's value alone.
    expect(root.querySelectorAll('a, [href], [target]')).toHaveLength(0);
    expect(root.innerHTML).not.toMatch(/https?:\/\//i);
    expect(root.innerHTML).not.toContain('pathofexile');

    // Typing cannot change it; the next view can.
    expect(area.readOnly).toBe(true);
    const one = fixtureView('trade-one-side');
    const r2 = await render(one);
    expect(field<HTMLTextAreaElement>(r2.root, 'Trade link').value).toBe(one.trade.link);
    // No link, no field.
    const empty = await render(fixtureView('trade-empty'));
    expect(empty.root.querySelector('textarea')).toBeNull();
    expect([...empty.root.querySelectorAll('label')].some((l) => text(l) === 'Trade link')).toBe(false);
  });

  it('trade_search_button_sends_its_command_without_an_address', async () => {
    const open = vi.spyOn(window, 'open').mockImplementation(() => null);
    const before = window.location.href;

    // Disabled with nothing to search: a click sends nothing, the reason beside it.
    const empty = await render(fixtureView('trade-empty'));
    const off = button(empty.root, 'Search on the trade site')!;
    expect(off.disabled).toBe(true);
    click(off);
    expect(empty.bridge.calls).toEqual([]);

    for (const name of ['trade-picking', 'trade-one-side', 'trade-default-weight']) {
      const { root, bridge } = await render(fixtureView(name));
      const search = button(root, 'Search on the trade site')!;
      // A native button, the window's primary, said to open the web browser, holding no address.
      expect(search.tagName, name).toBe('BUTTON');
      expect(search.getAttribute('type'), name).toBe('button');
      expect(search.closest('a'), name).toBeNull();
      expect(search.classList.contains('primary'), name).toBe(true);
      expect(search.disabled, name).toBe(false);
      expect(text(document.getElementById(search.getAttribute('aria-describedby')!)), name).toBe(
        'Opens the trade site in your web browser',
      );
      expect(search.querySelector('svg')?.getAttribute('aria-hidden'), name).toBe('true');
      expect(search.outerHTML, name).not.toMatch(/https?:|href|pathofexile|trade2/i);
      // The craft's own primary button is not on the page while the panel is open.
      expect(button(root, 'Start crafting'), name).toBeUndefined();

      click(search);
      expect(bridge.calls, name).toEqual([{ name: 'open_trade_search', args: {} }]);
    }
    expect(open).not.toHaveBeenCalled();
    expect(window.location.href).toBe(before);

    // Inside the app, the commands go out as declared: the search with no
    // argument at all, the selection with no field that names an address.
    const asked: Array<[string, unknown]> = [];
    mockIPC((cmd, payload) => {
      asked.push([cmd, payload]);
      return undefined;
    });
    await tauriBridge().command('open_trade_search', {});
    const sel = tradeOf('trade-picking').selection;
    await tauriBridge().command('set_trade_search', { search: sel });
    expect(asked).toEqual([
      ['open_trade_search', {}],
      ['set_trade_search', { search: sel }],
    ]);
    expect(Object.keys(sel).sort()).toEqual(['base', 'both_slots', 'class', 'item_level', 'open', 'picks']);
    for (const key of [...Object.keys(sel), ...sel.picks.flatMap((p) => Object.keys(p))]) {
      expect(key).not.toMatch(/url|uri|href|link|address|path|file|dir|location/i);
    }
  });
});

// jsdom applies no component stylesheet, so a computed style would read the
// same with the rule or without it: the test reads the component's source,
// as `../theme.test.ts` reads the theme's files. The live check
// `mod-centred` measures the rendered row.
describe('the modifier rows', () => {
  /** Each rule in a component's `<style>`, as its selectors and declarations (comments dropped). */
  function rules(source: string): Array<{ selectors: string[]; decls: Record<string, string> }> {
    const style = /<style>([\s\S]*?)<\/style>/.exec(source)?.[1] ?? '';
    const css = style.replace(/\/\*[\s\S]*?\*\//g, '');
    return [...css.matchAll(/([^{}]+)\{([^{}]*)\}/g)].map((m) => ({
      selectors: m[1].split(',').map((s) => s.trim().replace(/\s+/g, ' ')),
      decls: Object.fromEntries(
        m[2]
          .split(';')
          .map((d) => d.split(':').map((p) => p.trim()))
          .filter((p) => p.length === 2 && p[0] !== '')
          .map(([k, v]) => [k, v]),
      ),
    }));
  }
  /** The text-align a row's text gets: the last rule reaching `.name` inside the row's `label` that declares one. */
  function rowTextAlign(source: string): string | undefined {
    let align: string | undefined;
    for (const rule of rules(source)) {
      const reachesRow = rule.selectors.some((s) => /^(?:(?:\.mod-group|li|ul) )*(?:label )?\.name$/.test(s));
      if (reachesRow && rule.decls['text-align'] !== undefined) align = rule.decls['text-align'];
    }
    return align;
  }
  /** The text-align every `.name`, the not-searchable list's too, gets from a bare `.name` rule. */
  const bareNameAlign = (source: string): string | undefined =>
    rules(source)
      .filter((r) => r.selectors.includes('.name'))
      .map((r) => r.decls['text-align'])
      .filter((a) => a !== undefined)
      .pop();

  it('trade_modifier_text_is_centred_in_its_row', async () => {
    const source = new TextDecoder().decode(await readBytes('./trade/TradeModifiers.svelte'));
    expect(rowTextAlign(source)).toBe('center');
    // The not-searchable list keeps its left-aligned names.
    expect(bareNameAlign(source)).toBeUndefined();

    // Negative control: the rule as Live 1 saw it, the text left in its box.
    const before = `<style>
  label {
    display: flex;
  }

  .name {
    flex: 1;
    min-width: 0;
    color: var(--color-text-primary);
  }
</style>`;
    expect(rowTextAlign(before)).not.toBe('center');
  });

  /** A declaration as the last rule naming exactly `selector` gives it (a rule in `@media` counts too). */
  const decl = (source: string, selector: string, prop: string): string | undefined =>
    rules(source)
      .filter((r) => r.selectors.includes(selector))
      .map((r) => r.decls[prop])
      .filter((v) => v !== undefined)
      .pop();
  /** Every padding, margin and gap given as a bare length rather than through a theme token. */
  const bareLengths = (source: string): string[] =>
    rules(source).flatMap((r) =>
      Object.entries(r.decls)
        .filter(([k, v]) => /^(padding|margin|gap|row-gap|column-gap)(-|$)/.test(k) && /\d(px|em|rem)\b/.test(v) && !v.includes('var(--'))
        .map(([k, v]) => `${r.selectors.join(', ')} { ${k}: ${v} }`),
    );
  const onScale = (value: string | undefined): boolean => value !== undefined && value.includes('var(--space-');

  // The UI polish audit (owner, 2026-10-30): every box, row, field and
  // button of the panel and the League row has padding on the theme's
  // spacing scale, a row's text sits in the middle of its row, the weight
  // column keeps one right edge, and the controls on one row share a height.
  it('trade_panel_rows_have_padding_and_centred_text', async () => {
    const read = async (path: string) => new TextDecoder().decode(await readBytes(path));
    const mods = await read('./trade/TradeModifiers.svelte');
    const panel = await read('./trade/TradePanel.svelte');
    const league = await read('./settings/TradeSection.svelte');
    // The shared control looks, a plain stylesheet: wrapped so `rules` reads it as a component's block.
    const app = `<style>${await read('./app.css')}</style>`;

    // A modifier row: padding on the scale, its text centred across the row's height as well as its width.
    expect(onScale(decl(mods, '.mod-group li', 'padding'))).toBe(true);
    expect(decl(mods, 'label', 'align-items')).toBe('center');
    // The native box loses its default margins, so the text's column starts one gap after it.
    expect(decl(mods, 'label input', 'margin')).toBe('0');
    // The not-searchable rows are spaced as the searchable ones are.
    expect(decl(mods, '.not-searchable li', 'padding')).toBe(decl(mods, '.mod-group li', 'padding'));
    // A picked row's floor sits centred in the text's column: the row is a three-column grid
    // (checkbox, text, weight) the label shares, and the floor's box takes the middle one.
    expect(decl(mods, '.mod-group li', 'grid-template-columns')).toBe('auto minmax(0, 1fr) auto');
    expect(decl(mods, 'label', 'grid-template-columns')).toBe('subgrid');
    expect(decl(mods, 'label', 'grid-column')).toBe('1 / -1');
    expect(decl(mods, '.picked', 'grid-column')).toBe('2');
    expect(decl(mods, '.picked', 'align-items')).toBe('center');
    expect(decl(mods, '.picked', 'padding')).not.toMatch(/13px/);
    // The weight column: right-aligned tabular figures, and a row without the "default" tag keeps the tag's room
    // when the list has one, so every number ends on one right edge.
    expect(decl(mods, '.weight', 'text-align')).toBe('right');
    expect(decl(mods, '.weight', 'font-variant-numeric')).toBe('tabular-nums');
    const slot = rules(mods).find((r) => r.selectors.some((s) => /\.has-default li:not\(\[data-source='default'\]\) \.weight-cell::after$/.test(s)));
    expect(slot?.decls.visibility).toBe('hidden');
    expect(slot?.decls.content).toBe("'default'");
    expect(mods).toContain('class:has-default={anyDefault}');

    // The panel's boxes: the head, the scrolling area (both layouts), the Search bar, the small button, the link field.
    for (const box of ['.head', '.scroll', '.col', '.search-bar', '.small', '.link']) {
      expect(onScale(decl(panel, box, 'padding')), box).toBe(true);
    }
    // A group's legend sits as far above its first control as a heading does (the group's gap).
    expect(decl(panel, 'legend', 'padding')).toContain(decl(panel, '.group', 'gap'));

    // The League row: the theme's row height, and the select as tall as the refresh icon beside it.
    expect(decl(league, '.row', 'min-height')).toBe('var(--size-row-height)');
    expect(decl(league, '.pick', 'align-items')).toBe('stretch');
    expect(onScale(decl(league, '.pick', 'gap'))).toBe(true);

    // Buttons and fields centre their text and carry padding; no component overrides it.
    expect(decl(app, '.btn', 'justify-content')).toBe('center');
    expect(onScale(decl(app, '.btn', 'padding'))).toBe(true);
    expect(onScale(decl(app, '.field', 'padding'))).toBe(true);
    for (const source of [mods, panel, league]) {
      for (const button of ['.btn', '.back', '.search', '.small', '.icon']) {
        expect(decl(source, button, 'justify-content')).toBeUndefined();
        expect(decl(source, button, 'text-align')).toBeUndefined();
      }
      expect(bareLengths(source)).toEqual([]);
    }

    // Negative control: the rows as the audit found them, padded off the scale, text on the top line, the
    // radio's offset and the help line's pull in bare pixels.
    const before = `<style>
  .mod-group li {
    padding: 2px 0;
  }

  label {
    display: flex;
    align-items: baseline;
  }

  .choice input {
    margin-top: 4px;
  }

  .help {
    margin: -6px 0 0;
  }
</style>`;
    expect(onScale(decl(before, '.mod-group li', 'padding'))).toBe(false);
    expect(decl(before, 'label', 'align-items')).not.toBe('center');
    expect(bareLengths(before)).toEqual([
      '.mod-group li { padding: 2px 0 }',
      '.choice input { margin-top: 4px }',
      '.help { margin: -6px 0 0 }',
    ]);
  });
});
