// The requested art: elements the steel/masonry kit has no art for, each
// drawn by a marked CSS placeholder in Exile (`[data-art-placeholder="<id>"]`
// in `./steel.css`) until the owner delivers its file.
// `docs/waystone-roller/design/ASSET-REQUESTS.md` describes each one;
// `theme/materials/requested/manifest.json` lists them.
//
// A delivered file needs no code change: dropped into
// `theme/materials/requested/` under the name its entry lists, it is found
// here through `import.meta.glob` (which matches nothing while the folder
// holds only the manifest), and the page's root element gets
// `data-art-<id>` and one `--art-<file stem>` URL property per file, which
// the Exile rules draw in place of the placeholder. A request with several
// files is wired only when every one of them is there.

import MANIFEST from '../theme/materials/requested/manifest.json';

export interface ArtRequest {
  /** One token of `[a-z0-9-]`, prefixed `wr-`. */
  id: string;
  /** The file names the entry asks for, in `theme/materials/requested/`. */
  files: string[];
  /** The component and selector the art replaces. */
  element: string;
  /** `placeholder` until the file is delivered, then `delivered`. */
  status: string;
}

const DIR = '../theme/materials/requested/';

/** The delivered files by path, as Vite's URLs; empty until one is dropped in. */
const FILES = import.meta.glob(['../theme/materials/requested/*.png', '../theme/materials/requested/*.svg'], {
  eager: true,
  query: '?url',
  import: 'default',
}) as Record<string, string>;

/**
 * Marks `root` for every request whose files are all present in `files`
 * (path -> URL), so the Exile rules draw the art instead of the
 * placeholder. Returns the ids it wired, in manifest order.
 */
export function applyRequestedArt(
  root: HTMLElement,
  files: Readonly<Record<string, string>> = FILES,
  requests: readonly ArtRequest[] = MANIFEST.requests as ArtRequest[],
): string[] {
  const wired: string[] = [];
  for (const request of requests) {
    const urls = request.files.map((name) => files[`${DIR}${name}`]);
    if (urls.length === 0 || urls.some((url) => !url)) continue;
    request.files.forEach((name, i) => {
      root.style.setProperty(`--art-${name.replace(/\.(png|svg)$/, '')}`, `url("${urls[i]}")`);
    });
    root.setAttribute(`data-art-${request.id}`, '');
    wired.push(request.id);
  }
  return wired;
}
