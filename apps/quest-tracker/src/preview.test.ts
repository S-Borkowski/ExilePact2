import { isTauri } from '@tauri-apps/api/core';
import { afterEach, describe, expect, it } from 'vitest';
import { SCALE_MIN } from './ipc';
import {
  DEFAULT_PREVIEW_FRAME,
  PREVIEW_SCALES,
  SECTION_ROWS,
  SECTIONS_PREVIEW_FRAME,
  parsePreviewQuery,
  previewBridge,
  previewFixture,
} from './preview';
import { parseSettingsPreviewQuery } from './settings/preview';
import { type Rendered, renderApp } from './test-support';

let rendered: Rendered | null = null;

afterEach(() => {
  rendered?.destroy();
  rendered = null;
  clearScale();
});

function clearScale() {
  const root = document.documentElement;
  delete root.dataset.previewScale;
  root.style.removeProperty('--preview-scale');
  root.style.removeProperty('--page-zoom');
}

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
    const unknown = await previewFixture('no-such-frame', null);
    expect(unknown.view.status?.text).toMatch(/no frame "no-such-frame"/);
    // `toast` is no frame any more (owner, 2026-10-25, after Live 1).
    expect((await previewFixture('toast', null)).view.status?.text).toMatch(/no frame "toast"/);
  });

  it('preview_accepts_the_smallest_scale', async () => {
    // `0.75` is `SCALE_MIN`, the corner-safety runner's smallest scale; a
    // value the slider cannot reach is still ignored, on both pages.
    expect(PREVIEW_SCALES).toContain(String(SCALE_MIN));
    expect(parsePreviewQuery('?scale=0.75').scale).toBe('0.75');
    expect(parsePreviewQuery('?scale=0.5').scale).toBeNull();
    expect(parseSettingsPreviewQuery('?scale=0.75').scale).toBe('0.75');
    expect(parseSettingsPreviewQuery('?scale=0.5').scale).toBeNull();

    const root = document.documentElement;
    rendered = await renderApp(previewBridge('?frame=panel-next-up&scale=0.75', root));
    expect(root.dataset.previewScale).toBe('0.75');
    expect(root.style.getPropertyValue('--preview-scale')).toBe('0.75');
    expect(root.style.getPropertyValue('--page-zoom')).toBe('0.75');
    rendered.destroy();
    rendered = null;
    clearScale();

    // An unknown scale leaves the page unzoomed.
    rendered = await renderApp(previewBridge('?frame=panel-next-up&scale=0.5', root));
    expect(root.dataset.previewScale).toBeUndefined();
    expect(root.style.getPropertyValue('--preview-scale')).toBe('');
  });

  it('preview_shows_next_up_in_three_sections', async () => {
    // The dev-only view the section layout is checked on: every worth group,
    // two quests in each, from the campaign fixture's own rows.
    const { view } = await previewFixture(SECTIONS_PREVIEW_FRAME, 'edit');
    expect(view.frame).toBe('panel-next-up');
    expect(view.mode).toBe('edit');
    expect(view.status).toBeNull();
    expect(view.next_up.map((row) => row.worth)).toEqual(['yes', 'yes', 'optional', 'optional', 'no', 'no']);
    expect(view.next_up.map((row) => row.id)).toEqual(SECTION_ROWS.map(([id]) => id));
    expect(new Set(view.next_up.map((row) => row.id)).size).toBe(6);
    const campaign = (await previewFixture('panel-campaign', null)).view;
    const king = campaign.acts.flatMap((act) => act.quests).find((quest) => quest.id === 'fixture-king')!;
    expect(view.next_up.find((row) => row.id === 'fixture-king')).toEqual(king);
    // Without a mode it keeps the Next up fixture's own.
    expect((await previewFixture(SECTIONS_PREVIEW_FRAME, null)).view.mode).toBe('play');

    rendered = await renderApp(previewBridge(`?frame=${SECTIONS_PREVIEW_FRAME}&mode=play&theme=exile`));
    const overlay = rendered.root.querySelector('.overlay')!;
    expect(overlay.getAttribute('data-frame')).toBe('panel-next-up');
    expect(rendered.root.querySelectorAll('[data-quest]')).toHaveLength(6);
    expect(rendered.root.textContent).not.toContain('Preview: no frame');
  });
});
