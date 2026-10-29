// The overlay's browser preview (`ui-design.md` § "Browser preview"): the
// page renders a view fixture through `../bridge.ts`'s preview bridge, and
// `scene=bright` or `scene=dark` paints a flat white or near-black stand-in
// for the game behind the panel, in a window of the overlay's size, so its
// legibility over both can be checked. Never used inside the app.

export const PREVIEW_SCENES = ['bright', 'dark'] as const;
export type PreviewScene = (typeof PREVIEW_SCENES)[number];

/** The stand-in scene a query asks for, or null for none or an unknown one. */
export function previewScene(search: string): PreviewScene | null {
  const scene = new URLSearchParams(search).get('scene');
  return scene && (PREVIEW_SCENES as readonly string[]).includes(scene) ? (scene as PreviewScene) : null;
}

/** Marks the page as the preview, sized as the overlay's window, over the scene asked for. */
export function applyPreview(search: string, root: HTMLElement = document.documentElement): void {
  root.dataset.overlayPreview = previewScene(search) ?? 'none';
}
