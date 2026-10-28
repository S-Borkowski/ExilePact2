// The overlay (`overlay.html`): one line, the view's, which follows the next
// `ch:view`; it sends nothing, and the preview opens on the active craft.

import { afterEach, describe, expect, it, vi } from 'vitest';
import { DEFAULT_OVERLAY_PREVIEW_VIEW, previewBridge } from '../bridge';
import { type Rendered, fakeBridge, fixtureView, renderComponent, text } from '../test-support';
import Overlay from './Overlay.svelte';

let mounted: Rendered[] = [];

afterEach(() => {
  for (const r of mounted) r.destroy();
  mounted = [];
  vi.restoreAllMocks();
});

const ready = (root: HTMLElement) => root.querySelector('.overlay')?.getAttribute('data-visible') !== 'none';

describe('the overlay', () => {
  it('overlay_renders_the_next_step_line', async () => {
    const active = fixtureView('overlay-active');
    expect(active.overlay.visible).toBe(true);
    const bridge = fakeBridge(active);
    const r = await renderComponent(Overlay, { bridge }, ready);
    mounted.push(r);

    // The one line, as the app wrote it, and nothing else on the page.
    const overlay = r.root.querySelector('.overlay')!;
    expect(overlay.getAttribute('data-visible')).toBe('true');
    expect(r.root.querySelectorAll('p')).toHaveLength(1);
    expect(text(r.root.querySelector('.line'))).toBe(active.overlay.line);
    expect(active.overlay.line).toMatch(/next step/i);
    expect(r.root.querySelectorAll('button, input, a')).toHaveLength(0);

    // The line follows the next view; visibility is the shell's to act on.
    bridge.pushView({ ...active, overlay: { visible: true, line: 'Next step: use an Orb of Augmentation.' } });
    expect(text(r.root.querySelector('.line'))).toBe('Next step: use an Orb of Augmentation.');
    bridge.pushView(fixtureView('empty'));
    expect(overlay.getAttribute('data-visible')).toBe('false');
    expect(text(r.root.querySelector('.line'))).toBe(fixtureView('empty').overlay.line);

    // It never sends a command.
    expect(bridge.calls).toEqual([]);

    // Before any view: nothing drawn.
    const silent = await renderComponent(Overlay, { bridge: { ...fakeBridge(active), start: () => new Promise(() => {}) } }, () => true);
    mounted.push(silent);
    expect(silent.root.querySelector('.overlay')?.getAttribute('data-visible')).toBe('none');
    expect(silent.root.querySelector('.line')).toBeNull();

    // The preview opens on the active craft.
    expect(DEFAULT_OVERLAY_PREVIEW_VIEW).toBe('overlay-active');
    vi.spyOn(console, 'info').mockImplementation(() => {});
    const preview = await renderComponent(Overlay, { bridge: previewBridge('', DEFAULT_OVERLAY_PREVIEW_VIEW) }, ready);
    mounted.push(preview);
    expect(preview.root.querySelector('.overlay')?.getAttribute('data-visible')).toBe('true');
  });
});
