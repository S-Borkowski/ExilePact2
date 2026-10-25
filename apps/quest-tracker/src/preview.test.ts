import { isTauri } from '@tauri-apps/api/core';
import { afterEach, describe, expect, it } from 'vitest';
import { DEFAULT_PREVIEW_FRAME, parsePreviewQuery, previewBridge, previewFixture } from './preview';
import { type Rendered, renderApp } from './test-support';

let rendered: Rendered | null = null;

afterEach(() => {
  rendered?.destroy();
  rendered = null;
  delete document.documentElement.dataset.previewScale;
});

describe('the browser preview', () => {
  it('preview_mode_runs_without_tauri', async () => {
    // jsdom is a plain browser: no Tauri internals.
    expect(isTauri()).toBe(false);

    expect(parsePreviewQuery('')).toEqual({ frame: DEFAULT_PREVIEW_FRAME, mode: null, scale: null });
    expect(parsePreviewQuery('?frame=settings&mode=sideways&scale=3')).toEqual({ frame: 'settings', mode: null, scale: null });

    const root = document.documentElement;
    const bridge = previewBridge('?frame=quest-detail&mode=play&scale=2', root);
    expect(bridge.kind).toBe('preview');
    rendered = await renderApp(bridge);
    const overlay = rendered.root.querySelector('.overlay')!;
    expect(overlay.getAttribute('data-frame')).toBe('quest-detail');
    // The fixture is in edit mode; the query overrides it.
    expect(overlay.getAttribute('data-mode')).toBe('play');
    expect(root.dataset.previewScale).toBe('2');
    expect(rendered.root.textContent).toContain('Fixture: The King in the Mists');

    // Commands are logged, not sent anywhere.
    await bridge.command('set_mode', { mode: 'edit' });
    expect(bridge.calls).toEqual([{ name: 'set_mode', args: { mode: 'edit' } }]);
    rendered.destroy();
    rendered = null;

    // The brief's non-View frames have stand-ins, and an unknown frame says so.
    expect((await previewFixture('mode-edit', null)).view.mode).toBe('edit');
    expect((await previewFixture('toast', null)).toasts).toHaveLength(1);
    const unknown = await previewFixture('no-such-frame', null);
    expect(unknown.view.status?.text).toMatch(/no frame "no-such-frame"/);

    // The toast frame keeps its toast up for a screenshot.
    const toastBridge = previewBridge('?frame=toast');
    expect(toastBridge.holdToasts).toBe(true);
    rendered = await renderApp(toastBridge);
    expect(rendered.root.querySelectorAll('.toast')).toHaveLength(1);
  });
});
