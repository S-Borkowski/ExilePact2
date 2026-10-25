import { flushSync, mount, unmount } from 'svelte';
import { describe, expect, it } from 'vitest';
import brief from '../../../docs/quest-tracker/ui-design-brief.md?raw';
import defaults from './defaults.css?raw';
import StatusMark from './components/StatusMark.svelte';
import { contractList, cssName, rootProperties } from './contract';
import { QUEST_STATUSES } from './ipc';
import { STATUS_ICON } from './labels';
import { GLYPHS, iconGlyph, iconSvg } from './theme';

describe('the theme', () => {
  it('defaults_cover_every_token_key', () => {
    const required = contractList(brief, 'Token keys');
    const optional = contractList(brief, 'Optional token keys');
    expect(required.length).toBeGreaterThan(20);
    expect(optional.length).toBeGreaterThan(0);

    const declared = rootProperties(defaults);
    const missing = [...required, ...optional].map(cssName).filter((name) => !declared.has(name));
    expect(missing).toEqual([]);

    // Negative control: the parser does not find a key the file lacks.
    expect(rootProperties(':root { --color-panel-bg: #000; }').has(cssName('color.text-primary'))).toBe(false);
  });

  it('status_marks_differ_in_shape', () => {
    // Before the harvest: one glyph per status, every one a different character.
    const glyphs = QUEST_STATUSES.map((s) => iconGlyph(STATUS_ICON[s]));
    expect(new Set(glyphs).size).toBe(QUEST_STATUSES.length);
    for (const s of QUEST_STATUSES) expect(GLYPHS[STATUS_ICON[s]]).toBeTruthy();

    // As rendered (the harvested SVG where present): the mark's content, which
    // carries no colour, differs for every status.
    const shapes = new Map<string, string>();
    for (const status of QUEST_STATUSES) {
      const target = document.createElement('div');
      const mark = mount(StatusMark, { target, props: { status } });
      flushSync();
      const icon = target.querySelector('.status-mark .icon');
      expect(icon, status).not.toBeNull();
      expect(icon!.getAttribute('aria-label')).toBeTruthy();
      // The shape is the drawing inside, not the attributes that name it.
      const drawing = (icon!.querySelector('svg')?.innerHTML ?? icon!.textContent ?? '').trim();
      expect(drawing, status).not.toBe('');
      shapes.set(status, drawing);
      unmount(mark);
    }
    expect(new Set(shapes.values()).size).toBe(QUEST_STATUSES.length);

    // Every status icon id of the brief resolves to the harvested SVG when it exists.
    const iconIds = contractList(brief, 'Icon ids');
    for (const s of QUEST_STATUSES) expect(iconIds).toContain(STATUS_ICON[s]);
    const svg = iconSvg('status-done');
    if (svg !== null) {
      expect(svg).not.toMatch(/<title/);
      expect(svg).toMatch(/aria-hidden="true"/);
    }
  });
});
