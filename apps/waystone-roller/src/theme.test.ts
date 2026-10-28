import { describe, expect, it } from 'vitest';
import { listFiles, readBytes } from './test-support';

const same = (a: Uint8Array, b: Uint8Array) => a.length === b.length && a.every((byte, i) => byte === b[i]);

describe('the theme', () => {
  it('theme_tokens_match_quest_tracker', async () => {
    // The theme is the quest tracker's, copied byte for byte, so the two apps
    // read as one family; a change to either copy alone fails here.
    const ours = await readBytes('../theme/tokens.css');
    const theirs = await readBytes('../../quest-tracker/theme/tokens.css');
    expect(ours.length).toBeGreaterThan(1000);
    expect(same(ours, theirs), 'theme/tokens.css differs from the quest tracker\'s').toBe(true);

    const fonts = await listFiles('../theme/fonts');
    expect(fonts).toEqual(await listFiles('../../quest-tracker/theme/fonts'));
    expect(fonts).toContain('AlegreyaSans-Medium.woff2');
    expect(fonts).toContain('OFL.txt');
    for (const font of fonts) {
      const a = await readBytes(`../theme/fonts/${font}`);
      const b = await readBytes(`../../quest-tracker/theme/fonts/${font}`);
      expect(same(a, b), `theme/fonts/${font}`).toBe(true);
    }

    // Negative control: the comparison sees a one-byte change.
    const changed = ours.slice();
    changed[changed.length - 2] ^= 1;
    expect(same(changed, theirs)).toBe(false);
  });
});
