import { afterEach, describe, expect, it } from 'vitest';
import { flushSync } from 'svelte';
import type { View } from './ipc';
import { FIXTURE_NAMES, type Rendered, choice, click, fixtureView, modRow, renderView, setValue } from './test-support';

let rendered: Rendered | null = null;

afterEach(() => {
  rendered?.destroy();
  rendered = null;
});

async function render(view: View) {
  rendered?.destroy();
  rendered = await renderView(view);
  return { root: rendered.root, bridge: rendered.bridge };
}

const text = (el: Element | null) => (el?.textContent ?? '').replace(/\s+/g, ' ').trim();

const EMBER = 'monsters-deal-extra-ember';
const GLIMMER = 'monsters-have-glimmer-shields';
const CINDER = 'area-has-patches-of-cinder-ground';
const GILDED = 'monsters-are-gilded';

describe('the page', () => {
  it('renders_every_view_fixture', async () => {
    for (const name of FIXTURE_NAMES) {
      const view = fixtureView(name);
      const { root } = await render(view);
      const app = root.querySelector('.app');
      expect(app?.getAttribute('data-state'), name).toBe(view.result.state);

      expect(text(root.querySelector('h1'))).toBe('Waystone Roller');
      expect(text(root.querySelector('.source'))).toBe(`Mods from poe2db.tw, fetched ${view.data.fetched}`);

      // Every mod has a row, in the group its group and kind say, showing its
      // label, tier count and token, and its choice from the selection.
      expect(root.querySelectorAll('.mod')).toHaveLength(view.mods.length);
      for (const mod of view.mods) {
        const row = modRow(root, mod.id);
        const group = mod.group === 'desecrated' ? 'desecrated' : mod.kind === 'prefix' ? 'prefixes' : 'suffixes';
        expect(row.closest('.group')?.getAttribute('data-group'), mod.id).toBe(group);
        expect(text(row.querySelector('.label'))).toContain(mod.label);
        expect(text(row.querySelector('.tiers'))).toBe(`${mod.tiers} ${mod.tiers === 1 ? 'tier' : 'tiers'}`);
        if (mod.token !== null) expect(text(row.querySelector('.token'))).toBe(mod.token);
        if (mod.name !== null) expect(text(row)).toContain(mod.name);
        const expected = view.selection.want.includes(mod.id) ? 'want' : view.selection.avoid.includes(mod.id) ? 'avoid' : 'none';
        expect(row.getAttribute('data-choice'), mod.id).toBe(expected);
        expect(choice(root, mod.id, expected).checked, mod.id).toBe(true);
        // The choice is in the label column too, as a mark, not only a colour.
        const mark = text(row.querySelector('.choice-mark'));
        expect(mark).toBe(expected === 'want' ? '+' : expected === 'avoid' ? '−' : '');
      }

      // The footer: the string (or its placeholder), the meter, the hint, the status.
      const search = text(root.querySelector('.search-string'));
      if (view.result.state === 'ok') expect(search).toBe(view.result.regex);
      else expect(search).not.toContain('"');
      expect(text(root.querySelector('.meter'))).toContain(`${view.result.length} / 250`);
      expect(text(root.querySelector('.hint'))).toBe(`${view.hotkey} copies this while you play`);
      const status = root.querySelector('.status');
      if (view.status) {
        expect(text(status)).toContain(view.status.text);
        expect(status?.getAttribute('data-kind')).toBe(view.status.kind);
        if (view.status.kind === 'error') expect(text(status).startsWith('!')).toBe(true);
      } else {
        expect(text(status)).toBe('');
      }
      expect((root.querySelector<HTMLInputElement>('.shortcut input'))?.value).toBe(view.hotkey);

      // The filter row reflects the selection.
      expect(root.querySelector<HTMLSelectElement>('select.tier-min')?.value).toBe(String(view.selection.tier_min));
      expect(root.querySelector<HTMLSelectElement>('select.tier-max')?.value).toBe(String(view.selection.tier_max));
      for (const r of ['normal', 'magic', 'rare'] as const) {
        const button = root.querySelector(`button[data-rarity="${r}"]`);
        expect(button?.getAttribute('aria-pressed'), r).toBe(String(view.selection.rarity[r]));
      }
      expect(root.querySelector<HTMLInputElement>(`input[name="corruption"][value="${view.selection.corruption}"]`)?.checked).toBe(true);
      expect(root.querySelector<HTMLInputElement>(`input[name="want-mode"][value="${view.selection.want_mode}"]`)?.checked).toBe(true);
    }
  });

  it('want_and_avoid_send_update_selection', async () => {
    // ok-any: want ember and glimmer, avoid cinder.
    const { root, bridge } = await render(fixtureView('ok-any'));

    click(choice(root, CINDER, 'want'));
    expect(bridge.calls.pop()).toEqual({ name: 'update_selection', args: { patch: { want: [EMBER, GLIMMER, CINDER], avoid: [] } } });

    click(choice(root, EMBER, 'avoid'));
    expect(bridge.calls.pop()).toEqual({ name: 'update_selection', args: { patch: { want: [GLIMMER], avoid: [CINDER, EMBER] } } });

    click(choice(root, GLIMMER, 'none'));
    expect(bridge.calls.pop()).toEqual({ name: 'update_selection', args: { patch: { want: [EMBER], avoid: [CINDER] } } });
    expect(bridge.calls).toEqual([]);

    // The page waits for the shell's view: the rows follow the next `wr:view`.
    const next = fixtureView('ok-any');
    next.selection.want = [GLIMMER];
    next.selection.avoid = [CINDER, EMBER];
    bridge.pushView(next);
    expect(modRow(root, EMBER).getAttribute('data-choice')).toBe('avoid');
    expect(choice(root, EMBER, 'avoid').checked).toBe(true);

    // The filter row sends only the keys it changes.
    setValue(root.querySelector<HTMLSelectElement>('select.tier-min')!, '12');
    expect(bridge.calls.pop()).toEqual({ name: 'update_selection', args: { patch: { tier_min: 12 } } });
    setValue(root.querySelector<HTMLSelectElement>('select.tier-max')!, '15');
    expect(bridge.calls.pop()).toEqual({ name: 'update_selection', args: { patch: { tier_max: 15 } } });
    // The "from" list offers nothing above "to", the "to" list nothing below "from".
    const fromValues = [...root.querySelectorAll<HTMLOptionElement>('select.tier-min option')].map((o) => Number(o.value));
    expect(Math.max(...fromValues)).toBe(next.selection.tier_max);
    const toValues = [...root.querySelectorAll<HTMLOptionElement>('select.tier-max option')].map((o) => Number(o.value));
    expect(Math.min(...toValues)).toBe(next.selection.tier_min);

    click(root.querySelector('button[data-rarity="magic"]'));
    expect(bridge.calls.pop()).toEqual({ name: 'update_selection', args: { patch: { rarity: { normal: false, magic: true, rare: true } } } });
    click(root.querySelector('input[name="corruption"][value="corrupted"]'));
    expect(bridge.calls.pop()).toEqual({ name: 'update_selection', args: { patch: { corruption: 'corrupted' } } });
    click(root.querySelector('input[name="want-mode"][value="all"]'));
    expect(bridge.calls.pop()).toEqual({ name: 'update_selection', args: { patch: { want_mode: 'all' } } });

    // Clear choices empties both lists and nothing else.
    click(root.querySelector('button.clear-choices'));
    expect(bridge.calls.pop()).toEqual({ name: 'update_selection', args: { patch: { want: [], avoid: [] } } });
    expect(bridge.calls).toEqual([]);

    // The text filter and "Chosen only" hide rows and send nothing.
    setValue(root.querySelector<HTMLInputElement>('input.mod-filter')!, 'GLIMMER');
    expect([...root.querySelectorAll('.mod')].map((r) => r.getAttribute('data-mod-id'))).toEqual([GLIMMER]);
    setValue(root.querySelector<HTMLInputElement>('input.mod-filter')!, '');
    click(root.querySelector('input.chosen-only'));
    expect([...root.querySelectorAll('.mod')].map((r) => r.getAttribute('data-mod-id')).sort()).toEqual([CINDER, EMBER, GLIMMER].sort());
    expect(bridge.calls).toEqual([]);
  });

  it('too_long_shows_each_part_and_no_copy_success', async () => {
    const view = fixtureView('too-long');
    const { root, bridge } = await render(view);

    const meter = root.querySelector('.meter');
    expect(meter?.getAttribute('data-over')).toBe('true');
    expect(text(meter)).toContain(`${view.result.length} / 250`);
    expect(text(meter)).toContain(`over by ${view.result.length - 250}`);

    // Every part, in the search's order, with its text and its length; the
    // longest is marked in words.
    const parts = [...root.querySelectorAll('.parts .part')];
    expect(parts).toHaveLength(view.result.parts.length);
    view.result.parts.forEach((part, i) => {
      expect(parts[i].getAttribute('data-what')).toBe(part.what);
      expect(text(parts[i].querySelector('.part-text'))).toBe(part.text);
      expect(text(parts[i].querySelector('.part-length'))).toBe(String(part.length));
    });
    const longest = view.result.parts.reduce((a, b) => (b.length > a.length ? b : a));
    const marked = parts.filter((p) => text(p).includes('longest'));
    expect(marked.map((p) => p.getAttribute('data-what'))).toEqual([longest.what]);

    // Nothing to copy: no string, Copy disabled, and nothing claims a copy.
    expect(text(root.querySelector('.search-string'))).not.toContain(view.result.parts[0].text);
    const copy = root.querySelector<HTMLButtonElement>('button.copy')!;
    expect(copy.disabled).toBe(true);
    click(copy);
    expect(bridge.calls).toEqual([]);
    expect(text(root)).not.toContain('Copied the search');
    expect(text(root.querySelector('.status'))).toContain('Not copied: the search is 262 characters');

    // Negative control: the parts list is not shown for a search that fits.
    bridge.pushView(fixtureView('ok-any'));
    expect(root.querySelectorAll('.parts .part')).toHaveLength(0);
    expect(root.querySelector('.meter')?.getAttribute('data-over')).toBe('false');
  });

  it('unmatchable_row_disables_want_with_reason', async () => {
    const view = fixtureView('unmatchable');
    const { root, bridge } = await render(view);
    const gilded = view.mods.find((m) => m.id === GILDED)!;
    expect(gilded.token).toBeNull();

    expect(choice(root, GILDED, 'want').disabled).toBe(true);
    expect(choice(root, GILDED, 'avoid').disabled).toBe(false);
    expect(choice(root, GILDED, 'none').disabled).toBe(false);
    expect(text(modRow(root, GILDED).querySelector('.why-not'))).toBe(gilded.why_not!);
    expect(text(modRow(root, GILDED))).toContain('Cannot be matched');
    // The footer names the mods it cannot tell apart.
    expect(text(root.querySelector('.unmatchable-notice'))).toContain(gilded.label);
    expect(root.querySelector<HTMLButtonElement>('button.copy')!.disabled).toBe(true);

    // Avoid still works on it.
    click(choice(root, GILDED, 'avoid'));
    expect(bridge.calls.pop()).toEqual({ name: 'update_selection', args: { patch: { want: [EMBER], avoid: [GILDED] } } });

    // Negative control: a mod with a token has Want and no reason.
    expect(choice(root, EMBER, 'want').disabled).toBe(false);
    expect(modRow(root, EMBER).querySelector('.why-not')).toBeNull();
    expect(text(modRow(root, EMBER))).not.toContain('Cannot be matched');
  });

  it('copy_button_sends_copy_now', async () => {
    const { root, bridge } = await render(fixtureView('ok-any'));
    const copy = root.querySelector<HTMLButtonElement>('button.copy')!;
    expect(copy.disabled).toBe(false);
    click(copy);
    expect(bridge.calls).toEqual([{ name: 'copy_now', args: {} }]);

    // Negative control: nothing selected, nothing to copy.
    bridge.pushView(fixtureView('empty'));
    expect(root.querySelector<HTMLButtonElement>('button.copy')!.disabled).toBe(true);
    expect(text(root.querySelector('.search-string'))).toBe('Nothing selected yet');
  });

  it('shortcut_field_sends_set_hotkey', async () => {
    const view = fixtureView('hotkey-taken');
    const { root, bridge } = await render(view);
    expect(text(root.querySelector('.status'))).toContain('Shortcut Ctrl+Alt+W is taken by another program. Choose another one below.');

    const input = root.querySelector<HTMLInputElement>('.shortcut input')!;
    expect(input.value).toBe('Ctrl+Alt+W');
    setValue(input, '  Ctrl+Alt+R ');
    click(root.querySelector('.shortcut button[type="submit"]'));
    expect(bridge.calls).toEqual([{ name: 'set_hotkey', args: { hotkey: 'Ctrl+Alt+R' } }]);

    // Enter in the field submits too.
    bridge.calls.length = 0;
    setValue(input, 'Ctrl+Shift+F7');
    input.form!.requestSubmit();
    flushSync();
    expect(bridge.calls).toEqual([{ name: 'set_hotkey', args: { hotkey: 'Ctrl+Shift+F7' } }]);

    // Negative control: an empty field sends nothing.
    bridge.calls.length = 0;
    setValue(input, '   ');
    click(root.querySelector('.shortcut button[type="submit"]'));
    expect(bridge.calls).toEqual([]);

    // A new view with the shell's shortcut replaces what was typed.
    const next = fixtureView('ok-any');
    next.hotkey = 'Ctrl+Alt+R';
    bridge.pushView(next);
    expect(input.value).toBe('Ctrl+Alt+R');
    expect(text(root.querySelector('.hint'))).toBe('Ctrl+Alt+R copies this while you play');
  });
});
