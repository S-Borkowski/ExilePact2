// Requested art (`docs/crafting-helper/design/ASSET-REQUESTS.md`): elements
// of the Exile theme the steel/masonry kit has no art for. Each draws a CSS
// placeholder (`data-art-placeholder="<id>"`, `steel.css`) until the owner
// delivers its files into `theme/materials/requested/` under the names its
// manifest entry lists. Nothing in the code changes then: the glob below
// finds the files at the next build, `applyRequestedArt` sets one
// `--art-<file stem>` URL property per file found and, once every file of
// an entry is there, marks the document `data-art-<id>`, and the Exile
// rules draw the files in place of the placeholder. While the folder holds
// only `manifest.json`, the glob matches nothing and every placeholder
// stays.

import MANIFEST from '../theme/materials/requested/manifest.json';

/** One requested element: its id, the files it asks for, the element it replaces, and whether they arrived. */
export interface RequestedArt {
  id: string;
  files: string[];
  element: string;
  status: 'placeholder' | 'delivered';
}

export interface RequestedManifest {
  requests: RequestedArt[];
}

const FILES = import.meta.glob('../theme/materials/requested/*.{png,svg}', {
  eager: true,
  import: 'default',
  query: '?url',
}) as Record<string, string>;

/** The delivered files Vite found: path -> URL. Empty until the owner drops one in. */
export function requestedFiles(): Record<string, string> {
  return { ...FILES };
}

/**
 * Marks `root` for every delivered file: `--art-<stem>` for each file
 * found, and `data-art-<id>` for each entry whose files are all there.
 * Returns the ids marked, in manifest order.
 */
export function applyRequestedArt(
  root: HTMLElement,
  files: Readonly<Record<string, string>> = FILES,
  manifest: RequestedManifest = MANIFEST as RequestedManifest,
): string[] {
  const byName = new Map(Object.entries(files).map(([path, url]) => [path.replace(/^.*\//, ''), url]));
  const marked: string[] = [];
  for (const request of manifest.requests) {
    let all = true;
    for (const file of request.files) {
      const url = byName.get(file);
      if (url) root.style.setProperty(`--art-${file.replace(/\.(png|svg)$/, '')}`, `url("${url}")`);
      else all = false;
    }
    if (all) {
      root.setAttribute(`data-art-${request.id}`, '');
      marked.push(request.id);
    }
  }
  return marked;
}
