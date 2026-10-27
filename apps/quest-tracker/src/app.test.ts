import { afterEach, describe, expect, it } from 'vitest';
import brief from '../../../docs/quest-tracker/ui-design-brief.md?raw';
import { contractList } from './contract';
import { FRAMES, type View } from './ipc';
import { previewFixture } from './preview';
import { CONTROL_SELECTOR, type Rendered, fakeBridge, fixtureView, renderApp, rowChangeFor } from './test-support';

let rendered: Rendered | null = null;

afterEach(() => {
  rendered?.destroy();
  rendered = null;
});

async function render(view: View) {
  const bridge = fakeBridge(view);
  rendered = await renderApp(bridge);
  return { root: rendered.root, bridge };
}

const text = (el: Element) => el.textContent ?? '';

describe('the plain UI', () => {
  it('renders_every_frame_from_fixture', async () => {
    const frames = contractList(brief, 'Frame ids');
    expect(frames).toHaveLength(12);
    expect(frames).not.toContain('toast');

    for (const frame of frames) {
      const { view } = await previewFixture(frame, null);
      const bridge = fakeBridge(view);
      rendered = await renderApp(bridge);
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
        default:
          expect(frame.startsWith('empty-'), frame).toBe(true);
          expect(root.querySelector(`[data-empty="${frame}"]`), frame).not.toBeNull();
      }
      // The status line shows the view's status, except over the empty frame
      // whose body already states that condition (finish review F7).
      const statedByBody =
        (frame === 'empty-no-log' && view.status?.kind === 'error') ||
        (frame === 'empty-waiting' && view.status?.kind === 'info');
      if (view.status && statedByBody) {
        expect(root.querySelector('.status-line'), frame).toBeNull();
        expect(text(root.querySelector(`[data-empty="${frame}"]`)!).trim(), frame).not.toBe('');
      } else if (view.status) {
        expect(root.querySelector('.status-line')?.textContent, frame).toContain(view.status.text);
      }
      rendered.destroy();
      rendered = null;
    }
  });

  it('play_mode_has_no_controls', async () => {
    for (const frame of FRAMES) {
      const play = await render(fixtureView(frame, 'play'));
      // A row change too: the row it plays on and the live region that
      // announces it hold no control in play mode either.
      const view = fixtureView(frame, 'play');
      const row = [...view.next_up, ...view.acts.flatMap((a) => a.quests), ...(view.detail ? [view.detail.row] : [])][0];
      if (row) play.bridge.pushRowChange(rowChangeFor(row, 1, 'A step just done'));
      expect(play.root.querySelector('[aria-live="polite"]')?.textContent, frame).toContain(row ? 'A step just done' : '');
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
});
