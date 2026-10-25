// Finish review F7 and F13 (panel): the panel name is the page's heading, the
// status line is announced, and it is not drawn over an empty state that
// already says the same thing. The views are built here from a fixture's
// shape; nothing below depends on a fixture's names, counts or levels.

import { afterEach, describe, expect, it } from 'vitest';
import type { Frame, Mode, StatusLine, View } from './ipc';
import { type Rendered, fakeBridge, fixtureView, renderApp } from './test-support';

let rendered: Rendered | null = null;

afterEach(() => {
  rendered?.destroy();
  rendered = null;
});

async function render(view: View): Promise<HTMLElement> {
  rendered?.destroy();
  rendered = await renderApp(fakeBridge(view));
  return rendered.root;
}

/** A panel view with an invented character, in `mode`. */
function panelView(mode: Mode): View {
  const view = fixtureView('panel-campaign', mode);
  if (!view.character) throw new Error('panel-campaign has no character');
  view.character = { ...view.character, name: 'Header_Probe' };
  view.status = null;
  return view;
}

/** An empty frame's view with `status` set, in `mode`. */
function emptyView(frame: Frame, status: StatusLine | null, mode: Mode = 'play'): View {
  const view = fixtureView(frame, mode);
  view.frame = frame;
  view.status = status;
  return view;
}

describe('finish review: the panel header', () => {
  it('panel_name_is_h1', async () => {
    for (const mode of ['play', 'edit'] as const) {
      const root = await render(panelView(mode));
      const headings = root.querySelectorAll('.panel h1');
      expect(headings, mode).toHaveLength(1);
      expect(headings[0].textContent, mode).toContain('Header_Probe');
      expect(root.querySelector('.header')?.contains(headings[0]), mode).toBe(true);
    }
    // In edit mode the name still opens the character picker.
    const root = await render(panelView('edit'));
    expect(root.querySelector('.panel h1 button.who')).not.toBeNull();

    for (const mode of ['play', 'edit'] as const) {
      const none = await render(emptyView('empty-no-character', null, mode));
      const h1 = none.querySelector('.panel h1');
      expect(h1?.textContent?.trim(), mode).toBe('Quest tracker');
      expect(h1?.classList.contains('none'), mode).toBe(true);
    }
  });

  it('status_line_is_a_live_region', async () => {
    const view = panelView('play');
    view.status = { kind: 'warn', text: 'Reading the log failed: probe' };
    const root = await render(view);
    const line = root.querySelector('.status-line');
    expect(line).not.toBeNull();
    expect(line?.getAttribute('role')).toBe('status');
    expect(line?.textContent).toContain('Reading the log failed: probe');
  });

  it('status_line_hidden_when_body_shows_same_state', async () => {
    const cases: Array<[Frame, StatusLine]> = [
      ['empty-no-log', { kind: 'error', text: 'No Client.txt found. Set the log path in Settings.' }],
      ['empty-waiting', { kind: 'info', text: 'Waiting for the game to write to the log.' }],
    ];
    for (const [frame, status] of cases) {
      for (const mode of ['play', 'edit'] as const) {
        const root = await render(emptyView(frame, status, mode));
        expect(root.querySelector('.overlay')?.getAttribute('data-frame'), `${frame} ${mode}`).toBe(frame);
        expect(root.querySelector('.status-line'), `${frame} ${mode}`).toBeNull();
        expect(root.querySelector('[role="status"]'), `${frame} ${mode}`).toBeNull();
        // The body still states the condition.
        expect(root.querySelector('.body')?.textContent?.trim(), `${frame} ${mode}`).not.toBe('');
      }
    }
  });

  it('status_line_kept_for_other_problems', async () => {
    const cases: Array<[Frame, StatusLine]> = [
      // A problem the waiting body does not show.
      ['empty-waiting', { kind: 'error', text: 'Progress is not saved: probe' }],
      ['empty-waiting', { kind: 'warn', text: 'Command line: probe' }],
      // A lesser note over the no-log body is not the no-log condition.
      ['empty-no-log', { kind: 'warn', text: 'Command line: probe' }],
      // The same kinds over a body that does not state them.
      ['empty-no-character', { kind: 'info', text: 'Waiting for the game to write to the log.' }],
      ['empty-act-done', { kind: 'error', text: 'No Client.txt found. Set the log path in Settings.' }],
    ];
    for (const [frame, status] of cases) {
      const root = await render(emptyView(frame, status));
      const line = root.querySelector('.status-line');
      expect(line, `${frame} ${status.kind}`).not.toBeNull();
      expect(line?.getAttribute('role')).toBe('status');
      expect(line?.textContent).toContain(status.text);
    }
    const panel = panelView('edit');
    panel.status = { kind: 'info', text: 'Waiting for the game to write to the log.' };
    const root = await render(panel);
    expect(root.querySelector('.status-line')?.textContent).toContain('Waiting for the game');
  });
});
