import { flushSync } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import bridgeSource from './bridge.ts?raw';
import collapsedSource from './components/Collapsed.svelte?raw';
import questRowSource from './components/QuestRow.svelte?raw';
import { rootProperties } from './contract';
import { type RowChange, SCALE_MAX, SCALE_MIN, SCALE_STEP, type View, viewProblems } from './ipc';
import { previewBridge } from './preview';
import { ROW_HIGHLIGHT_MS } from './rowchange';
import { type Rendered, fakeBridge, fixtureView, readText, renderApp, rowChangeFor } from './test-support';

// The owner's fixes after Live 2: a highlight on every live row change that
// shows with Windows' animations off too, the window drag from the edit-mode
// header, and the settings redesign (opacity from 0, scale as an Auto switch
// plus a slider, no dock side). Views are cloned fixtures.

let rendered: Rendered | null = null;

afterEach(() => {
  rendered?.destroy();
  rendered = null;
  vi.useRealTimers();
  delete (window as { matchMedia?: unknown }).matchMedia;
});

async function render(view: View, fakeTimers = false) {
  const bridge = fakeBridge(view);
  rendered = await renderApp(bridge);
  // Rendered on real timers (renderApp waits on them); the highlight runs on fake ones.
  if (fakeTimers) vi.useFakeTimers();
  return { root: rendered.root, bridge };
}

function close(): void {
  rendered?.destroy();
  rendered = null;
  vi.useRealTimers();
}

/** Windows' animation setting as the webview reports it. */
function setReducedMotion(reduce: boolean): void {
  Object.defineProperty(window, 'matchMedia', {
    configurable: true,
    value: (query: string) => ({ matches: reduce && query.includes('prefers-reduced-motion: reduce'), media: query }),
  });
}

function advance(ms: number): void {
  vi.advanceTimersByTime(ms);
  flushSync();
}

const rowOf = (root: HTMLElement, id: string) => root.querySelector<HTMLElement>(`[data-quest="${id}"]`)!;
const lit = (el: Element | null) => el?.hasAttribute('data-highlight') ?? false;
const highlighted = (root: HTMLElement) => [...root.querySelectorAll('[data-highlight]')];

/** The `var(--…)` names a component's `<style>` reads that look like theme tokens. */
function tokenUses(source: string, from: string): string[] {
  const style = source.slice(source.indexOf('<style>'));
  const block = style.slice(style.indexOf(from));
  return [...new Set([...block.matchAll(/var\((--(?:color|opacity|motion)-[a-z0-9-]+)/g)].map((m) => m[1]))];
}

async function knownTokens(): Promise<Set<string>> {
  const tokens = rootProperties(await readText('../theme/tokens.css'));
  for (const name of rootProperties(await readText('./defaults.css'))) tokens.add(name);
  return tokens;
}

/** A settings view in `mode`, with `patch` over the fixture's settings. */
function settingsView(mode: 'play' | 'edit', patch: Partial<View['settings']> = {}): View {
  const view = fixtureView('settings', mode);
  view.frame = 'settings';
  Object.assign(view.settings, patch);
  return view;
}

function rowByLabel(root: HTMLElement, label: string): HTMLElement {
  const row = [...root.querySelectorAll<HTMLElement>('.row')].find((r) => r.querySelector('.label, dt')?.textContent?.trim() === label);
  expect(row, label).toBeTruthy();
  return row!;
}

function change(input: HTMLInputElement, value: string): void {
  input.value = value;
  input.dispatchEvent(new Event('change', { bubbles: true }));
  flushSync();
}

describe('the row highlight', () => {
  it('row_change_highlights_the_row', async () => {
    const view = fixtureView('panel-next-up', 'play');
    const king = view.next_up.find((r) => r.id === 'fixture-king')!;
    const lute = view.next_up.find((r) => r.id === 'fixture-lute')!;
    setReducedMotion(false);
    const { root, bridge } = await render(view, true);
    expect(highlighted(root)).toHaveLength(0);

    bridge.pushRowChange(rowChangeFor(king, 1, 'Cross the bridge'));
    // On at once, on this row only, beside the step's own transition.
    expect(lit(rowOf(root, king.id))).toBe(true);
    expect(rowOf(root, king.id).dataset.change).toBe('out');
    expect(lit(rowOf(root, lute.id)), 'an unchanged row').toBe(false);
    expect(highlighted(root)).toHaveLength(1);

    // Held about 2.5 s, past the step's transition, then cleared.
    expect(ROW_HIGHLIGHT_MS).toBe(2500);
    advance(ROW_HIGHLIGHT_MS - 1);
    expect(rowOf(root, king.id).dataset.change).toBeUndefined();
    expect(lit(rowOf(root, king.id))).toBe(true);
    advance(1);
    expect(lit(rowOf(root, king.id))).toBe(false);

    // A second change restarts the hold; it never blinks off between the two.
    bridge.pushRowChange(rowChangeFor(king, 2, 'Step A'));
    advance(2000);
    bridge.pushRowChange(rowChangeFor(king, 3, 'Step B'));
    expect(lit(rowOf(root, king.id))).toBe(true);
    advance(2000);
    expect(lit(rowOf(root, king.id)), '4 s after the first, 2 s after the second').toBe(true);
    advance(ROW_HIGHLIGHT_MS - 2000);
    expect(lit(rowOf(root, king.id))).toBe(false);
    advance(10_000);
    expect(vi.getTimerCount()).toBe(0);
    close();

    // Edit mode: the row is a button, and is highlighted the same way.
    const edit = await render(fixtureView('panel-next-up', 'edit'), true);
    edit.bridge.pushRowChange(rowChangeFor(lute, 1, null));
    expect(rowOf(edit.root, lute.id).tagName).toBe('BUTTON');
    expect(lit(rowOf(edit.root, lute.id))).toBe(true);
    close();

    // A row not on screen (its act folded) is announced, never highlighted.
    const folded = fixtureView('panel-campaign', 'play');
    folded.acts[0].expanded = false;
    folded.acts[0].quests = [];
    const off = await render(folded, true);
    off.bridge.pushRowChange(rowChangeFor(king, 1, 'Cross the bridge'));
    expect(highlighted(off.root)).toHaveLength(0);
    expect(off.root.querySelector('[aria-live="polite"]')?.textContent).toContain('Cross the bridge');

    // The tint is colour only, in existing tokens: the new status colour.
    const uses = tokenUses(questRowSource, 'The highlight: the row');
    expect(uses).toContain('--color-status-in-progress');
    expect(uses).toContain('--color-status-done');
    const known = await knownTokens();
    for (const name of uses) expect(known.has(name), name).toBe(true);
    expect(questRowSource).toMatch(/\.row\[data-highlight\]::after\s*{\s*opacity: 0\.2;\s*transition: none;/);
    expect(questRowSource).toMatch(/transition: opacity var\(--row-highlight-out-ms\)/);
    // A finished row's muted text keeps AA on a lighter tint (the audit's finding).
    expect(questRowSource).toMatch(/\.row\.finished\[data-highlight\]::after\s*{\s*opacity: 0\.14;/);
  });

  it('reduced_motion_change_still_highlights', async () => {
    const view = fixtureView('panel-next-up', 'play');
    const king = view.next_up.find((r) => r.id === 'fixture-king')!;
    setReducedMotion(true);
    const { root, bridge } = await render(view, true);
    bridge.pushRowChange(rowChangeFor(king, 1, 'Cross the bridge'));
    const row = rowOf(root, king.id);
    // No movement: no phase, the new step at once; the highlight shows anyway.
    expect(row.dataset.change).toBeUndefined();
    expect(root.querySelector('.leaving, .entering')).toBeNull();
    expect([...row.querySelectorAll('.step')].map((s) => s.textContent)).toEqual(['Face the king']);
    expect(lit(row)).toBe(true);
    advance(ROW_HIGHLIGHT_MS);
    expect(lit(rowOf(root, king.id))).toBe(false);
    // Nothing in the highlight's rules is turned off under reduced motion.
    const reduced = questRowSource.slice(questRowSource.indexOf('@media (prefers-reduced-motion: reduce)'));
    expect(reduced).not.toMatch(/highlight/);
    close();

    // Negative control: motion on, the same change both plays and highlights.
    setReducedMotion(false);
    const on = await render(fixtureView('panel-next-up', 'play'), true);
    on.bridge.pushRowChange(rowChangeFor(king, 1, 'Cross the bridge'));
    expect(rowOf(on.root, king.id).dataset.change).toBe('out');
    expect(lit(rowOf(on.root, king.id))).toBe(true);
  });

  it('collapsed_badge_shows_a_change', async () => {
    const badge = (root: HTMLElement) => root.querySelector<HTMLElement>('.badge')!;
    const quest = (id: number, kind: RowChange['kind']): RowChange => ({
      id,
      kind,
      quest_id: 'fixture-king',
      step_done: kind === 'step_done' ? 'Cross the bridge' : null,
      step_next: kind === 'step_done' ? 'Face the king' : null,
      text: `Change ${id}.`,
    });

    for (const mode of ['play', 'edit'] as const) {
      const { root, bridge } = await render(fixtureView('collapsed', mode), true);
      expect(lit(badge(root)), mode).toBe(false);
      bridge.pushRowChange(quest(1, 'step_done'));
      expect(badge(root).dataset.highlight, mode).toBe('step_done');
      advance(2000);
      // A second change restarts the hold, in the new change's colour.
      bridge.pushRowChange(quest(2, 'quest_done'));
      expect(badge(root).dataset.highlight).toBe('quest_done');
      advance(2000);
      expect(lit(badge(root)), `${mode}: 2 s after the second`).toBe(true);
      advance(ROW_HIGHLIGHT_MS - 2000);
      expect(lit(badge(root)), mode).toBe(false);
      // Still announced.
      expect(root.querySelector('[aria-live="polite"]')?.textContent).toContain('Change 2.');
      close();
    }

    // Negative control: a change from before the badge was shown is not played.
    const panel = fixtureView('panel-next-up', 'play');
    const { root, bridge } = await render(panel, true);
    bridge.pushRowChange(rowChangeFor(panel.next_up[0], 1, 'Cross the bridge'));
    bridge.pushView(fixtureView('collapsed', 'play'));
    expect(root.querySelector('.badge')).not.toBeNull();
    expect(lit(badge(root))).toBe(false);

    // The tint uses existing tokens only.
    const known = await knownTokens();
    const uses = tokenUses(collapsedSource, '.badge[data-highlight]');
    expect(uses).toContain('--color-status-done');
    for (const name of uses) expect(known.has(name), name).toBe(true);
  });
});

describe('the window drag', () => {
  it('edit_header_drags_the_window', async () => {
    const press = (target: Element, button = 0) => {
      target.dispatchEvent(new MouseEvent('pointerdown', { bubbles: true, button }));
      flushSync();
    };

    const edit = await render(fixtureView('panel-campaign', 'edit'));
    const header = edit.root.querySelector<HTMLElement>('header.header')!;
    // A press on the bar, outside its buttons: one drag.
    press(header.querySelector('.spacer')!);
    expect(edit.bridge.drags).toBe(1);
    press(header.querySelector('.chip')!);
    expect(edit.bridge.drags).toBe(2);
    // On a button, or with another mouse button: no drag, and the button still works.
    const settings = header.querySelector<HTMLElement>('button[aria-label="Settings"]')!;
    press(settings);
    press(header.querySelector('button.who span')!);
    press(header, 2);
    expect(edit.bridge.drags).toBe(2);
    settings.click();
    expect(edit.bridge.calls).toEqual([{ name: 'open_frame', args: { frame: 'settings' } }]);
    // The rest of the panel never drags.
    press(edit.root.querySelector('.body')!);
    expect(edit.bridge.drags).toBe(2);
    close();

    // Play mode never drags: the window is click-through there.
    const play = await render(fixtureView('panel-campaign', 'play'));
    press(play.root.querySelector('header.header')!);
    press(play.root.querySelector('header.header .name')!);
    expect(play.bridge.drags).toBe(0);
    close();

    // The preview has no window to move: a no-op that sends nothing.
    const preview = previewBridge('');
    expect(() => preview.startDrag()).not.toThrow();
    expect(preview.calls).toEqual([]);
    // Inside the app, the bridge asks the Tauri window to start dragging.
    expect(bridgeSource).toMatch(/getCurrentWindow\(\)\s*\.startDragging\(\)/);
  });
});

describe('the settings frame after Live 2', () => {
  it('opacity_slider_reaches_zero', async () => {
    const { root, bridge } = await render(settingsView('edit', { opacity: 0 }));
    const slider = rowByLabel(root, 'Background opacity').querySelector<HTMLInputElement>('input[type="range"]')!;
    expect([slider.min, slider.max, slider.step]).toEqual(['0', '1', '0.05']);
    expect(rowByLabel(root, 'Background opacity').querySelector('.value')?.textContent).toBe('0%');
    expect(slider.style.getPropertyValue('--fill')).toBe('0%');
    const help = rowByLabel(root, 'Background opacity').nextElementSibling!;
    expect(help.classList.contains('help')).toBe(true);
    expect(help.textContent).not.toMatch(/floor|30%/i);
    expect(help.textContent).toMatch(/border/);
    change(slider, '0');
    expect(bridge.calls).toEqual([{ name: 'update_settings', args: { patch: { opacity: 0 } } }]);
    close();

    // At 0 the panel keeps its border, and in edit mode the accent.
    for (const mode of ['play', 'edit'] as const) {
      const panel = fixtureView('panel-campaign', mode);
      panel.settings.opacity = 0;
      const { root: r } = await render(panel);
      expect(r.querySelector<HTMLElement>('.overlay')!.style.getPropertyValue('--panel-opacity')).toBe('0');
      expect(r.querySelector('.panel.cut')).not.toBeNull();
      expect(r.querySelector('.panel.edit') !== null, mode).toBe(mode === 'edit');
      close();
    }
    const panelSource = await readText('./components/Panel.svelte');
    expect(panelSource).toMatch(/\.panel \{[^}]*--ring-color: var\(--color-panel-border\)/);
    expect(panelSource).toMatch(/\.panel\.edit \{[^}]*--ring-color: var\(--color-edit-accent\)/);

    // The theme's floor is 0, in the harvested files and the defaults alike.
    const theme = JSON.parse(await readText('../theme/theme.json')) as Record<string, unknown>;
    expect(theme['opacity.min']).toBe(0);
    expect(await readText('../theme/tokens.css')).toMatch(/--opacity-min: 0;/);
    expect(await readText('./defaults.css')).toMatch(/--opacity-min: 0;/);
  });

  it('scale_is_an_auto_switch_and_a_slider', async () => {
    expect([SCALE_MIN, SCALE_MAX, SCALE_STEP]).toEqual([0.75, 2, 0.05]);

    // Auto on: the switch is on, the slider rests at 1×, the value reads Auto.
    let { root, bridge } = await render(settingsView('edit', { scale: 'auto' }));
    const auto = rowByLabel(root, 'Auto scale').querySelector<HTMLInputElement>('input[type="checkbox"][role="switch"]')!;
    let slider = rowByLabel(root, 'Scale').querySelector<HTMLInputElement>('input[type="range"]')!;
    expect(auto.checked).toBe(true);
    expect([slider.min, slider.max, slider.step]).toEqual(['0.75', '2', '0.05']);
    expect(slider.value).toBe('1');
    expect(rowByLabel(root, 'Scale').querySelector('.value')?.textContent).toBe('Auto');
    // Moving the slider sends a fixed factor (the shell turns Auto off with it).
    change(slider, '1.25');
    // Turning Auto off starts from 1×.
    auto.click();
    flushSync();
    expect(bridge.calls).toEqual([
      { name: 'update_settings', args: { patch: { scale: 1.25 } } },
      { name: 'update_settings', args: { patch: { scale: 1 } } },
    ]);
    close();

    // A fixed factor: Auto off, the slider on it, and turning Auto on sends "auto".
    ({ root, bridge } = await render(settingsView('edit', { scale: 1.5 })));
    slider = rowByLabel(root, 'Scale').querySelector<HTMLInputElement>('input[type="range"]')!;
    expect(rowByLabel(root, 'Auto scale').querySelector<HTMLInputElement>('input')!.checked).toBe(false);
    expect(slider.value).toBe('1.5');
    expect(slider.style.getPropertyValue('--fill')).toBe('60%');
    expect(rowByLabel(root, 'Scale').querySelector('.value')?.textContent).toBe('1.5×');
    rowByLabel(root, 'Auto scale').querySelector<HTMLInputElement>('input')!.click();
    flushSync();
    expect(bridge.calls).toEqual([{ name: 'update_settings', args: { patch: { scale: 'auto' } } }]);
    // No dropdown is left for the scale.
    expect(root.querySelector('select')).toBeNull();
    close();

    // Play mode lists the value, with no control.
    for (const [scale, text] of [
      ['auto', 'Auto'],
      [2, '2×'],
      [0.75, '0.75×'],
    ] as const) {
      ({ root } = await render(settingsView('play', { scale })));
      expect(rowByLabel(root, 'Scale').querySelector('dd')?.textContent).toBe(text);
      expect(root.querySelector('input, select')).toBeNull();
      close();
    }

    // The IPC check takes "auto" or a number, and nothing else.
    const fixed = settingsView('edit', { scale: 1.33 });
    expect(viewProblems(fixed)).toEqual([]);
    const bad = settingsView('edit');
    (bad.settings as { scale: unknown }).scale = 'big';
    expect(viewProblems(bad)).toEqual(['view.settings.scale: "big" is not "auto" or a number']);
  });

  it('settings_have_no_dock_row', async () => {
    for (const mode of ['play', 'edit'] as const) {
      const { root } = await render(settingsView(mode));
      expect(root.textContent, mode).not.toMatch(/dock/i);
      expect(root.querySelector('[data-dock]'), mode).toBeNull();
      // The play-mode list matches the controls: one entry per setting.
      if (mode === 'play') {
        expect([...root.querySelectorAll('dt')].map((d) => d.textContent?.trim())).toEqual([
          'Background opacity',
          'Scale',
          'Show skipped quests',
          'Only worth-it quests',
          'Read the screen',
          'Hide under game panels',
          'Edit-mode hotkey',
          'Log file',
        ]);
      }
      close();
    }

    // No frame carries a dock attribute either.
    const { root } = await render(fixtureView('panel-campaign', 'edit'));
    expect(root.querySelector('[data-dock]')).toBeNull();
    close();

    // The fixtures carry no `dock`, and a view that still had one would be a drift.
    expect('dock' in fixtureView('settings').settings).toBe(false);
    const old = fixtureView('settings') as unknown as { settings: Record<string, unknown> };
    old.settings.dock = 'right';
    expect(viewProblems(old)).toEqual(['view.settings.dock: not in the TypeScript types']);
  });
});
