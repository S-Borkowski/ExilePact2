import { flushSync, mount, unmount } from 'svelte';
import { describe, expect, it } from 'vitest';
import brief from '../../../docs/quest-tracker/ui-design-brief.md?raw';
import indexHtml from '../index.html?raw';
import defaults from './defaults.css?raw';
import Icon from './components/Icon.svelte';
import { contractList, cssName, rootProperties } from './contract';
import { THEME, applyDerivedTokens, dropShadowFilter, iconGlyph, iconSvg } from './theme';
import EXILE from '../theme/exile.json';
import { readText } from './test-support';

// The styled layer's sources, as text. Svelte and TypeScript `?raw` imports
// keep their text under Vitest; CSS is read from disk (`readText`).
const SVELTE = import.meta.glob('./**/*.svelte', { query: '?raw', import: 'default', eager: true }) as Record<string, string>;
const TS = import.meta.glob(['./**/*.ts', '!./**/*.test.ts'], { query: '?raw', import: 'default', eager: true }) as Record<
  string,
  string
>;
const SVGS = import.meta.glob('../theme/icons/*.svg', { query: '?raw', import: 'default', eager: true }) as Record<string, string>;
const FONT_FILES = Object.keys(import.meta.glob('../theme/fonts/*', { query: '?url', import: 'default', eager: true }));

/** The CSS the app ships besides the components: the plain defaults and the styled layer's shared rules. */
async function appCss(): Promise<Record<string, string>> {
  return { './defaults.css': defaults, './styled.css': await readText('./styled.css'), './approved.css': await readText('./approved.css'), '../theme/exile.css': await readText('../theme/exile.css') };
}

/** Every custom property a text reads through `var(--name)` with no fallback of its own. */
function varsRead(text: string): Set<string> {
  const out = new Set<string>();
  for (const m of text.matchAll(/var\(\s*(--[a-z0-9-]+)\s*\)/gi)) out.add(m[1]);
  return out;
}

/** Every custom property a text sets: `--name:` in CSS, `style:--name=` in Svelte, `setProperty('--name')` in TypeScript. */
function varsSet(text: string): Set<string> {
  const out = new Set<string>();
  for (const m of text.matchAll(/(?:^|[\s;{(])(--[a-z0-9-]+)\s*:/gim)) out.add(m[1]);
  for (const m of text.matchAll(/style:(--[a-z0-9-]+)/g)) out.add(m[1]);
  for (const m of text.matchAll(/setProperty\(\s*['"](--[a-z0-9-]+)['"]/g)) out.add(m[1]);
  return out;
}

// Namespace names and this machine's dev server are not loads.
const NOT_REMOTE = /^(?:https?:\/\/www\.w3\.org\/|https?:\/\/(?:127\.0\.0\.1|localhost)[:/])/i;

/** Every remote reference in a text: a URL with a network scheme, or a protocol-relative `url()`, `src` or `href`. */
function remoteRefs(text: string): string[] {
  const out: string[] = [];
  for (const m of text.matchAll(/\b(?:https?|wss?|ftp):\/\/[^\s"'()<>]+/gi)) if (!NOT_REMOTE.test(m[0])) out.push(m[0]);
  for (const m of text.matchAll(/(?:url\(\s*|\b(?:src|href)\s*=\s*)['"]?\/\/[^\s"'()<>]*/gi)) out.push(m[0]);
  for (const m of text.matchAll(/@import\b[^;]*/gi)) out.push(m[0]);
  return out;
}

/** The targets of every `url(...)` in a CSS text. */
function cssUrls(css: string): string[] {
  return [...css.matchAll(/url\(\s*['"]?([^'")]+?)['"]?\s*\)/g)].map((m) => m[1]);
}

function mountIcon(id: string) {
  const target = document.createElement('div');
  const icon = mount(Icon, { target, props: { id, size: '16px' } });
  flushSync();
  return { target, destroy: () => unmount(icon) };
}

describe('the styled layer', () => {
  it('harvested_tokens_cover_every_key', async () => {
    const required = contractList(brief, 'Token keys');
    const optional = contractList(brief, 'Optional token keys');
    expect(required.length).toBeGreaterThan(20);

    // The harvest wrote every contract key into theme.json ...
    expect(THEME, 'theme/theme.json').not.toBeNull();
    const theme = THEME!;
    expect(required.filter((key) => !(key in theme))).toEqual([]);
    expect(typeof theme['opacity.default']).toBe('number');
    expect(theme['opacity.default']).toBeGreaterThan(0);
    expect(theme['opacity.default']).toBeLessThanOrEqual(1);

    // ... and tokens.css declares a custom property for every key theme.json has.
    const tokens = await readText('../theme/tokens.css');
    const declared = rootProperties(tokens);
    expect(Object.keys(theme).map(cssName).filter((name) => !declared.has(name))).toEqual([]);
    // An optional key is in both files or in neither (then the default applies).
    for (const key of optional) expect(declared.has(cssName(key)), key).toBe(key in theme);

    // The styled layer reads only tokens the harvest wrote, the optional keys
    // it did not design (their plain defaults), or the drop shadows derived from
    // the harvested shadows: no token invented on the code side.
    const sources = { ...SVELTE, ...TS, ...(await appCss()) };
    const local = new Set<string>();
    for (const [path, text] of Object.entries(sources)) if (path !== './defaults.css') for (const n of varsSet(text)) local.add(n);
    const allowed = new Set([...Object.keys(theme).map(cssName), ...Object.keys(EXILE).map(cssName), ...optional.map(cssName), '--drop-panel']);
    const invented: string[] = [];
    for (const [path, text] of Object.entries(sources)) {
      if (path === './defaults.css') continue;
      for (const name of varsRead(text)) if (!allowed.has(name) && !local.has(name)) invented.push(`${path}: ${name}`);
    }
    expect(invented).toEqual([]);
    // The styled layer does read the harvest's own extras, not only the contract keys.
    const read = new Set(Object.values(sources).flatMap((text) => [...varsRead(text)]));
    for (const extra of ['--chamfer-panel', '--color-edit-tint', '--color-row-active-bg', '--font-family-caps']) expect(read.has(extra), extra).toBe(true);

    // Negative controls: a theme without a key is caught, and the scan finds a key a file lacks.
    const short: Record<string, unknown> = { ...theme };
    delete short['color.panel-bg'];
    expect(required.filter((key) => !(key in short))).toEqual(['color.panel-bg']);
    expect(rootProperties(':root { --color-panel-bg: #000; }').has(cssName('color.text-primary'))).toBe(false);
    expect(varsRead('a { color: var(--color-made-up); }').has('--color-made-up')).toBe(true);
  });

  it('every_icon_id_has_an_svg', () => {
    const ids = contractList(brief, 'Icon ids');
    expect(ids).toHaveLength(41);
    expect(ids.filter((id) => iconSvg(id) === null)).toEqual([]);

    for (const [path, svg] of Object.entries(SVGS)) {
      expect(svg, path).toMatch(/^\s*<svg\b/);
      // No external reference, script, raster or event handler (the brief's rule).
      expect(remoteRefs(svg), path).toEqual([]);
      expect(svg, path).not.toMatch(/<script|<image\b|<foreignObject|\son[a-z]+\s*=/i);
      expect(svg, path).not.toMatch(/(?:xlink:)?href\s*=\s*["'](?!#)/i);
      // Single-colour icons draw in currentColor; the rest carry their own colours.
      if (!/currentColor/.test(svg)) expect(svg, `${path} is multi-colour`).toMatch(/#[0-9a-f]{3,8}\b|rgba?\(/i);
    }

    // As rendered: every id draws its SVG, inline, unlabelled (the element around it carries the name).
    for (const id of ids) {
      const { target, destroy } = mountIcon(id);
      const svg = target.querySelector(`[data-icon="${id}"] svg`);
      expect(svg, id).not.toBeNull();
      expect(svg!.getAttribute('aria-hidden')).toBe('true');
      expect(svg!.querySelector('title'), id).toBeNull();
      expect(target.querySelector('.glyph'), id).toBeNull();
      destroy();
    }

    // Negative control: an id the harvest has no file for has no SVG.
    expect(iconSvg('no-such-icon')).toBeNull();
  });

  it('missing_asset_falls_back_to_plain', async () => {
    // Icons: with no harvested SVG at all, every contract id still draws a glyph,
    // and the six status glyphs differ in shape.
    const ids = contractList(brief, 'Icon ids');
    for (const id of ids) {
      expect(iconSvg(id, {}), id).toBeNull();
      expect(iconGlyph(id).trim(), id).not.toBe('');
    }
    const statusGlyphs = ids.filter((id) => id.startsWith('status-')).map(iconGlyph);
    expect(new Set(statusGlyphs).size).toBe(6);

    // As rendered: an id with no SVG draws its glyph ...
    const missing = mountIcon('reward-not-harvested');
    expect(missing.target.querySelector('svg')).toBeNull();
    expect(missing.target.querySelector('.glyph')?.textContent).toBe('◆');
    missing.destroy();
    // ... and, as the negative control, a harvested id draws its SVG and no glyph.
    const present = mountIcon('reward-spirit');
    expect(present.target.querySelector('svg')).not.toBeNull();
    expect(present.target.querySelector('.glyph')).toBeNull();
    present.destroy();

    // Tokens: every custom property the UI reads has a plain default in
    // defaults.css (or is set by the UI itself), so a missing tokens.css or a
    // key it lacks leaves the plain value, never an unset property.
    const plain = rootProperties(defaults);
    const sources = { ...SVELTE, ...TS, ...(await appCss()) };
    const local = new Set<string>();
    for (const [path, text] of Object.entries(sources)) if (path !== './defaults.css') for (const n of varsSet(text)) local.add(n);
    const unbacked: string[] = [];
    for (const [path, text] of Object.entries(sources)) {
      for (const name of varsRead(text)) if (!plain.has(name) && !local.has(name)) unbacked.push(`${path}: ${name}`);
    }
    expect(unbacked).toEqual([]);
    // The scan sees the styled layer's shared rules, not an empty file.
    expect(varsRead(sources['./styled.css']).size).toBeGreaterThan(20);

    // The derived drop shadows: with no harvested theme nothing is set and the
    // defaults apply; with it, each harvested box-shadow becomes a filter.
    const root = document.createElement('div');
    applyDerivedTokens(root, null);
    expect(root.style.getPropertyValue('--drop-panel')).toBe('');
    expect(plain.has('--drop-panel')).toBe(true);
    applyDerivedTokens(root, { 'shadow.panel': '0 0 0 1px rgba(0,0,0,0.63), 0 4px 12px rgba(0,0,0,0.5)' });
    expect(root.style.getPropertyValue('--drop-panel')).toBe('drop-shadow(0 0 1px rgba(0,0,0,0.63)) drop-shadow(0 4px 12px rgba(0,0,0,0.5))');
    expect(dropShadowFilter('inset 0 1px 0 #fff, 2px 3px 4px #000')).toBe('drop-shadow(2px 3px 4px #000)');
  });

  it('fonts_and_assets_are_local_only', async () => {
    const tokens = await readText('../theme/tokens.css');

    // Every url() in tokens.css is a bundled WOFF2 under theme/fonts/.
    const urls = cssUrls(tokens);
    expect(urls.length).toBeGreaterThan(0);
    for (const url of urls) {
      expect(url, url).not.toMatch(/^(?:[a-z][a-z0-9+.-]*:|\/)/i);
      expect(url, url).toMatch(/\.woff2$/);
      expect(FONT_FILES, url).toContain(`../theme/${url}`);
    }
    // Each @font-face names a family the theme uses, from a local file only.
    const faces = [...tokens.matchAll(/@font-face\s*\{([^}]*)\}/g)].map((m) => m[1]);
    expect(faces.length).toBeGreaterThan(0);
    const families = new Set([THEME?.['font.family'], THEME?.['font.family-caps']].filter(Boolean));
    for (const face of faces) {
      const family = face.match(/font-family:\s*["']?([^"';]+)["']?;/)?.[1];
      expect(families, face).toContain(family);
      expect(face).not.toMatch(/\blocal\(/);
    }
    // theme.json's font files exist, and the licence ships beside them.
    for (const [key, value] of Object.entries(THEME ?? {})) {
      if (key.startsWith('font.file')) expect(FONT_FILES, key).toContain(`../theme/${value}`);
    }
    expect(FONT_FILES.some((f) => /OFL|LICEN[CS]E/i.test(f))).toBe(true);

    // No remote load anywhere in the UI: tokens, the app's CSS, every
    // component and module, and the page itself.
    const texts: Record<string, string> = { '../theme/tokens.css': tokens, '../index.html': indexHtml, ...SVELTE, ...TS, ...(await appCss()) };
    const remote = Object.entries(texts).flatMap(([path, text]) => remoteRefs(text).map((ref) => `${path}: ${ref}`));
    expect(remote).toEqual([]);

    // Negative controls: the scan catches a web font, a protocol-relative URL and an @import.
    expect(remoteRefs('src: url("https://fonts.gstatic.com/s/a.woff2")')).toHaveLength(1);
    expect(remoteRefs('src: url(//cdn.example.com/a.woff2)')).toHaveLength(1);
    expect(remoteRefs('@import "theme.css";')).toHaveLength(1);
    expect(remoteRefs('<svg xmlns="http://www.w3.org/2000/svg">')).toEqual([]);
    expect(cssUrls('src: url("fonts/a.woff2") format("woff2")')).toEqual(['fonts/a.woff2']);
  });
});
