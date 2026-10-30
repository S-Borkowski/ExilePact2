// The main window never scrolls past its content. Measured in headless Edge
// (`scripts/scroll-height.mjs`): the window scrolled up to 2,222 px past the
// views with long lists, because each visually hidden span (`app.css`'s
// `.visually-hidden`, `position: absolute`) had no positioned ancestor, so
// its containing block was the page itself. Laid out at its static place far
// down a scroll pane's list, the span stood outside the pane's clipping and
// stretched the document below the window.
//
// jsdom does no layout, so this pins the structure, in the cascade-reading
// style of `src/ui-spacing.test.ts`: every element positioned out of the flow
// either has a positioned box at or inside its nearest scroll pane (so the
// pane holds it), or is pinned to its containing block's top left corner (so
// wherever that block is, the element adds nothing below it). A scroll pane
// is any element a rule gives `overflow` or `overflow-y` `auto` or `scroll`,
// at any width: the rules inside `@media` count here, since a pane at one
// width is a pane.

import { afterEach, describe, expect, it } from 'vitest';
import './app.css';
import { type Rendered, fixtureView, renderView } from './test-support';

/** Every style rule on the page, in order, with whether it sits inside a grouping rule such as `@media`. */
function styleRules(): Array<{ rule: CSSStyleRule; grouped: boolean }> {
  const out: Array<{ rule: CSSStyleRule; grouped: boolean }> = [];
  const walk = (rules: CSSRuleList, grouped: boolean) => {
    for (const rule of Array.from(rules)) {
      if (rule instanceof CSSStyleRule) out.push({ rule, grouped });
      else if ('cssRules' in rule) walk((rule as CSSGroupingRule).cssRules, true);
    }
  };
  for (const sheet of Array.from(document.styleSheets)) walk(sheet.cssRules, false);
  return out;
}

/** A selector's specificity as one comparable number (ids, classes, types); `:where()` counts nothing. */
function specificity(selector: string): number {
  const s = selector
    .replace(/:where\((?:[^()]|\([^()]*\))*\)/g, '')
    .replace(/::?[a-z-]+\(/gi, (m) => (m.startsWith('::') ? ' ' : ' .x '))
    .replace(/[()]/g, ' ');
  const ids = (s.match(/#[\w-]+/g) ?? []).length;
  const classes = (s.match(/\.[\w-]+|\[[^\]]*\]|(?<!:):[a-z-]+/gi) ?? []).length;
  const types = (s.match(/(?:^|[\s>+~])[a-z][\w-]*/gi) ?? []).length;
  return ids * 10000 + classes * 100 + types;
}

function matches(el: Element, selector: string): boolean {
  try {
    return el.matches(selector);
  } catch {
    // A selector jsdom cannot parse matches nothing here.
    return false;
  }
}

/**
 * Every value of `property` a rule matching `el` declares, the winner of the
 * plain rules' cascade first (`undefined` when none declares it), then the
 * values of the rules inside `@media` and the like. An inline style wins.
 */
function declarations(el: Element, property: string): { winner: string | undefined; grouped: string[] } {
  const inline = (el as HTMLElement).style?.getPropertyValue(property).trim();
  let best: { spec: number; order: number; value: string } | undefined;
  const grouped: string[] = [];
  styleRules().forEach(({ rule, grouped: inGroup }, order) => {
    const value = rule.style.getPropertyValue(property).trim();
    if (!value) return;
    const spec = Math.max(-1, ...rule.selectorText.split(',').map((s) => (matches(el, s) ? specificity(s) : -1)));
    if (spec < 0) return;
    if (inGroup) grouped.push(value);
    else if (!best || spec > best.spec || (spec === best.spec && order >= best.order)) best = { spec, order, value };
  });
  return { winner: inline || best?.value, grouped };
}

const OUT_OF_FLOW = new Set(['absolute', 'fixed']);
const SCROLLS = /^(auto|scroll)$/;

/** Whether any rule, at any width, lets `el` scroll vertically. */
function isScrollPane(el: Element): boolean {
  for (const property of ['overflow', 'overflow-y']) {
    const { winner, grouped } = declarations(el, property);
    // `overflow: hidden auto` scrolls on y, its second value.
    const values = [winner, ...grouped].filter((v): v is string => !!v).map((v) => v.split(/\s+/).at(-1) ?? v);
    if (values.some((v) => SCROLLS.test(v))) return true;
  }
  return false;
}

/** Whether a rule gives `el` a position other than static, so it is a containing block for absolute boxes. */
function isPositioned(el: Element): boolean {
  const position = declarations(el, 'position').winner;
  return position !== undefined && position !== 'static';
}

/** Whether `el` is placed at its containing block's top left corner (`top` and `left`, or `inset`, of 0). */
function isPinned(el: Element): boolean {
  const zero = (v: string | undefined) => v !== undefined && /^0(px)?$/.test(v);
  const inset = declarations(el, 'inset').winner?.split(/\s+/);
  if (inset && zero(inset[0]) && zero(inset[3] ?? inset[1] ?? inset[0])) return true;
  return zero(declarations(el, 'top').winner) && zero(declarations(el, 'left').winner);
}

/**
 * Why each element under `root` positioned out of the flow could stretch the
 * page past its scroll pane, or nothing: it is neither held by a positioned
 * box at or inside its nearest scroll pane nor pinned to its containing
 * block's corner.
 */
function escapes(root: Element): string[] {
  const problems: string[] = [];
  for (const el of Array.from(root.querySelectorAll('*'))) {
    const position = declarations(el, 'position').winner;
    if (!position || !OUT_OF_FLOW.has(position)) continue;
    if (isPinned(el)) continue;
    let held = false;
    for (let at = el.parentElement; at && at !== root; at = at.parentElement) {
      if (position === 'absolute' && isPositioned(at)) {
        held = true;
        break;
      }
      if (isScrollPane(at)) break;
    }
    if (!held) {
      const name = `${el.tagName.toLowerCase()}${el.id ? `#${el.id}` : ''}.${Array.from(el.classList).join('.')}`;
      problems.push(`${name} (${position}): no positioned box holds it in its scroll pane, and it is not pinned`);
    }
  }
  return problems;
}

describe('the main view never scrolls past its content', () => {
  const mounted: Rendered[] = [];
  afterEach(() => {
    while (mounted.length) mounted.pop()?.destroy();
    for (const el of Array.from(document.querySelectorAll('style[data-probe]'))) el.remove();
  });

  it('no_positioned_element_escapes_its_scroll_pane', async () => {
    // The views with the longest lists: the ring's and the belt's target
    // choices, and the trade panel's modifier lists.
    let seen = 0;
    const problems: string[] = [];
    for (const name of ['craft-planning', 'craft-side-full', 'craft-guiding', 'trade-picking', 'trade-default-weight']) {
      const r = await renderView(fixtureView(name));
      mounted.push(r);
      seen += r.root.querySelectorAll('.visually-hidden').length;
      problems.push(...escapes(r.root).map((p) => `${name}: ${p}`));
      r.destroy();
      mounted.pop();
    }
    // The views draw the hidden spans this is about.
    expect(seen).toBeGreaterThan(20);
    expect(problems).toEqual([]);
    // Five full views, each element's cascade read rule by rule: alone it
    // takes a few seconds, and beside the cargo suites of a parallel verify
    // it outran vitest's 5 s default. The limit only bounds a hang.
  }, 60_000);

  it('an_unpinned_absolute_box_in_a_scroll_pane_is_caught', () => {
    // The negative control: the check finds the shape the bug had, and
    // passes it once a positioned box inside the pane holds it.
    const style = document.createElement('style');
    style.dataset.probe = '';
    style.textContent = `
      .probe-pane { overflow-y: auto; }
      .probe-held { position: relative; }
      .probe-out { position: absolute; }
    `;
    document.head.appendChild(style);
    const root = document.createElement('div');
    root.innerHTML = '<div class="probe-pane"><ul><li><span class="probe-out">why</span></li></ul></div>';
    document.body.appendChild(root);
    try {
      expect(escapes(root)).toHaveLength(1);
      root.querySelector('li')?.classList.add('probe-held');
      expect(escapes(root)).toEqual([]);
    } finally {
      root.remove();
    }
  });
});
