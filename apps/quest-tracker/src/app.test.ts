import { afterEach, describe, expect, it, vi } from 'vitest';
import brief from '../../../docs/quest-tracker/ui-design-brief.md?raw';
import { contractList } from './contract';
import { FRAMES, type View } from './ipc';
import { previewFixture } from './preview';
import { pushToast, type ShownToast, MAX_TOASTS, TOAST_MS } from './toasts';
import { CONTROL_SELECTOR, type Rendered, fakeBridge, fixtureToast, fixtureView, renderApp } from './test-support';

let rendered: Rendered | null = null;

afterEach(() => {
  rendered?.destroy();
  rendered = null;
  vi.useRealTimers();
});

async function render(view: View, holdToasts = false) {
  const bridge = fakeBridge(view, holdToasts);
  rendered = await renderApp(bridge);
  return { root: rendered.root, bridge };
}

const text = (el: Element) => el.textContent ?? '';

describe('the plain UI', () => {
  it('renders_every_frame_from_fixture', async () => {
    const frames = contractList(brief, 'Frame ids');
    expect(frames).toHaveLength(13);

    for (const frame of frames) {
      const { view, toasts } = await previewFixture(frame, null);
      const bridge = fakeBridge(view, true);
      rendered = await renderApp(bridge);
      for (const toast of toasts) bridge.pushToast(toast);
      const root = rendered.root;
      const overlay = root.querySelector('.overlay');
      expect(overlay?.getAttribute('data-frame'), frame).toBe(view.frame);
      expect(view.status, `${frame} is a known frame`).not.toEqual(expect.objectContaining({ text: expect.stringMatching(/^Preview: no frame/) }));

      switch (frame) {
        case 'panel-campaign':
        case 'mode-play':
        case 'mode-edit': {
          expect(overlay?.getAttribute('data-mode')).toBe(frame === 'mode-edit' ? 'edit' : frame === 'mode-play' ? 'play' : view.mode);
          const rows = view.acts.flatMap((a) => (a.expanded ? a.quests : []));
          expect(rows.length).toBeGreaterThan(0);
          for (const row of rows) expect(text(root), row.name).toContain(row.name);
          for (const act of view.acts) expect(text(root)).toContain(`${act.done}/${act.total}`);
          break;
        }
        case 'panel-next-up':
          expect(view.next_up.length).toBeGreaterThan(0);
          for (const row of view.next_up) {
            expect(root.querySelector(`[data-quest="${row.id}"]`), row.id).not.toBeNull();
            if (row.current_step) expect(text(root)).toContain(row.current_step);
          }
          break;
        case 'collapsed':
          expect(root.querySelector('.badge .count')?.textContent).toBe(String(view.available_worth_count));
          expect(root.querySelector('.panel')).toBeNull();
          break;
        case 'quest-detail':
          expect(view.detail).not.toBeNull();
          expect(text(root)).toContain(view.detail!.row.name);
          expect(text(root)).toContain(view.detail!.worth_reason);
          for (const step of view.detail!.steps) expect(root.querySelector(`.step[data-state="${step.state}"]`)).not.toBeNull();
          for (const reward of view.detail!.row.rewards) expect(text(root)).toContain(reward.text);
          break;
        case 'character-picker':
          expect(view.characters.length).toBeGreaterThan(0);
          for (const c of view.characters) expect(text(root)).toContain(c.name);
          expect(root.querySelectorAll('[aria-current="true"]')).toHaveLength(view.characters.filter((c) => c.active).length);
          break;
        case 'settings':
          expect(text(root)).toContain('Settings');
          expect(root.querySelector('input[type="range"]') ?? root.querySelector('.values')).not.toBeNull();
          break;
        case 'toast':
          expect(toasts).toHaveLength(1);
          expect(root.querySelector('.toast')?.textContent).toContain(toasts[0].text);
          break;
        default:
          expect(frame.startsWith('empty-'), frame).toBe(true);
          expect(root.querySelector(`[data-empty="${frame}"]`), frame).not.toBeNull();
      }
      if (view.status) expect(text(root)).toContain(view.status.text);
      rendered.destroy();
      rendered = null;
    }
  });

  it('play_mode_has_no_controls', async () => {
    for (const frame of FRAMES) {
      const play = await render(fixtureView(frame, 'play'));
      // A toast too: the stack below the panel holds no control in play mode either.
      play.bridge.pushToast(fixtureToast());
      expect(play.root.querySelector('.overlay')?.getAttribute('data-mode')).toBe('play');
      const controls = [...play.root.querySelectorAll(CONTROL_SELECTOR)].map((el) => el.outerHTML.slice(0, 80));
      expect(controls, `${frame} in play mode`).toEqual([]);
      rendered!.destroy();
      rendered = null;

      // Negative control: the same frame in edit mode does have controls.
      const edit = await render(fixtureView(frame, 'edit'));
      expect(edit.root.querySelectorAll(CONTROL_SELECTOR).length, `${frame} in edit mode`).toBeGreaterThan(0);
      rendered!.destroy();
      rendered = null;
    }
  });

  it('edit_mode_is_distinguishable', async () => {
    for (const frame of FRAMES) {
      const play = await render(fixtureView(frame, 'play'));
      const playOverlay = play.root.querySelector('.overlay')!;
      const playMarks = {
        mode: playOverlay.getAttribute('data-mode'),
        tag: play.root.querySelector('.mode-tag'),
        editPanel: play.root.querySelector('.panel.edit'),
        editBadge: play.root.querySelector('button.badge'),
      };
      rendered!.destroy();
      rendered = null;

      const edit = await render(fixtureView(frame, 'edit'));
      const editOverlay = edit.root.querySelector('.overlay')!;
      expect(playMarks.mode).toBe('play');
      expect(editOverlay.getAttribute('data-mode')).toBe('edit');
      if (frame === 'collapsed') {
        // The badge alone: a button with the accent border in edit mode.
        expect(playMarks.editBadge).toBeNull();
        expect(edit.root.querySelector('button.badge')).not.toBeNull();
      } else {
        // A visible "Edit" tag in the header and the accent-tinted panel.
        expect(playMarks.tag, frame).toBeNull();
        expect(playMarks.editPanel, frame).toBeNull();
        expect(edit.root.querySelector('.mode-tag')?.textContent?.trim(), frame).toBe('Edit');
        expect(edit.root.querySelector('.panel.edit'), frame).not.toBeNull();
      }
      rendered!.destroy();
      rendered = null;
    }
  });

  it('toasts_stack_at_most_three', async () => {
    // The stack itself: the newest three, a repeated id replaced, not doubled.
    const t = (n: number): ShownToast => ({ key: `qt-${n}`, kind: 'available', text: `toast ${n}`, icon: 'status-available' });
    let stack: ShownToast[] = [];
    for (let n = 1; n <= 5; n++) stack = pushToast(stack, t(n));
    expect(MAX_TOASTS).toBe(3);
    expect(stack.map((s) => s.key)).toEqual(['qt-3', 'qt-4', 'qt-5']);
    expect(pushToast(stack, t(5)).map((s) => s.key)).toEqual(['qt-3', 'qt-4', 'qt-5']);

    // As rendered from `qt:toast` events: three at most, below the panel, gone after ~3 s.
    const { root, bridge } = await render(fixtureView('panel-next-up', 'play'));
    vi.useFakeTimers();
    for (let n = 1; n <= 5; n++) bridge.pushToast({ ...fixtureToast(), id: n, text: `Toast ${n}` });
    const shown = [...root.querySelectorAll('.toast')].map((el) => el.textContent?.trim());
    expect(shown).toEqual(['Toast 3', 'Toast 4', 'Toast 5']);

    const panel = root.querySelector('.panel')!;
    const list = root.querySelector('.toasts')!;
    expect(panel.compareDocumentPosition(list) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();

    vi.advanceTimersByTime(TOAST_MS - 1);
    bridge.pushView(fixtureView('panel-next-up', 'play'));
    expect(root.querySelectorAll('.toast')).toHaveLength(3);
    vi.advanceTimersByTime(1);
    bridge.pushView(fixtureView('panel-next-up', 'play'));
    expect(root.querySelectorAll('.toast')).toHaveLength(0);
  });
});
