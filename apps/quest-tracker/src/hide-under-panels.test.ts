import { afterEach, describe, expect, it } from 'vitest';
import { viewProblems, type View } from './ipc';
import { type Rendered, fakeBridge, fixtureView, renderApp } from './test-support';

// Hiding under game panels is always on since 0.1.1: the settings frame has
// no "Hide under game panels" row in either mode, the view's settings carry
// no `hide_under_panels`, and a view that still had one would be a drift.

let rendered: Rendered | null = null;

afterEach(() => {
  rendered?.destroy();
  rendered = null;
});

function settingsView(mode: 'play' | 'edit'): View {
  const view = fixtureView('settings', mode);
  view.frame = 'settings';
  return view;
}

async function render(view: View) {
  const bridge = fakeBridge(view);
  rendered = await renderApp(bridge);
  return { root: rendered.root, bridge };
}

function labels(root: HTMLElement): string[] {
  return [...root.querySelectorAll<HTMLElement>('.row .label, .row dt')].map((l) => l.textContent?.trim() ?? '');
}

describe('hiding under game panels, always on', () => {
  it('settings_has_no_hide_under_panels_row', async () => {
    for (const mode of ['edit', 'play'] as const) {
      const { root, bridge } = await render(settingsView(mode));
      // Control: the settings frame is drawn, with the row that sat above it.
      expect(labels(root), mode).toContain('Read the screen');
      expect(labels(root), mode).not.toContain('Hide under game panels');
      expect(root.textContent, mode).not.toMatch(/hide under game panels/i);
      expect(root.textContent, mode).not.toMatch(/game panel is open/);
      expect(bridge.calls, mode).toEqual([]);
      rendered?.destroy();
      rendered = null;
    }
  });

  it('views_carry_no_hide_under_panels', () => {
    expect('hide_under_panels' in fixtureView('settings').settings).toBe(false);
    const old = fixtureView('settings') as unknown as { settings: Record<string, unknown> };
    old.settings.hide_under_panels = true;
    expect(viewProblems(old)).toEqual(['view.settings.hide_under_panels: not in the TypeScript types']);
    // Control: the fixture as it is has no problem.
    expect(viewProblems(fixtureView('settings'))).toEqual([]);
  });
});
