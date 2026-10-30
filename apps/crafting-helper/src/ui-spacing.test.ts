// The spacing and alignment the UI review fixed (`ui-design.md` § "Spacing
// and alignment"), pinned on the styles that ship. jsdom does no layout, so
// nothing here measures a box: each test reads the declarations that reach
// an element from the stylesheets Vitest injects (the components' `<style>`
// blocks and `app.css`, `vite.config.ts`'s `css` option), resolves them
// against the shared tokens and checks the numbers and the DOM structure.
//
// jsdom's own `getComputedStyle` drops a shorthand that holds a `var()`
// (`padding: 0 calc(var(--space-unit) * 3)` reads as `0`), so the cascade
// is taken here: every style rule whose selector matches, the most specific
// last. Rules inside `@media` are left out (jsdom has no viewport); none of
// them sets a control's padding, margin or alignment.

import { afterEach, describe, expect, it } from 'vitest';
import './app.css';
import CurrencyPicker from './components/CurrencyPicker.svelte';
import SettingsWindow from './settings/SettingsWindow.svelte';
import type { SettingsWindowBridge } from './settings/bridge';
import {
  CATALOGUE,
  type Rendered,
  fakeSettingsBridge,
  fixtureView,
  readBytes,
  renderComponent,
  renderView,
  settingsFixture,
} from './test-support';

/** The shared tokens (`theme/tokens.css`) and `app.css`'s own `:root` names, as `--name` → value. */
async function tokens(): Promise<Map<string, string>> {
  const text = new TextDecoder().decode(await readBytes('../theme/tokens.css'));
  const out = new Map<string, string>();
  for (const m of text.matchAll(/(--[a-z0-9-]+)\s*:\s*([^;]+);/gi)) out.set(m[1], m[2].trim());
  for (const rule of styleRules()) {
    if (rule.selectorText !== ':root') continue;
    for (const name of Array.from(rule.style)) if (name.startsWith('--')) out.set(name, rule.style.getPropertyValue(name).trim());
  }
  return out;
}

/** Every plain style rule on the page, in order (grouping rules such as `@media` left out). */
function styleRules(): CSSStyleRule[] {
  const out: CSSStyleRule[] = [];
  for (const sheet of Array.from(document.styleSheets)) {
    for (const rule of Array.from(sheet.cssRules)) if (rule instanceof CSSStyleRule) out.push(rule);
  }
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

/** The four sides a `padding` or `margin` shorthand gives, its values split at the top level. */
function sides(value: string): [string, string, string, string] {
  const parts: string[] = [];
  let depth = 0;
  let word = '';
  for (const ch of value.trim()) {
    if (ch === '(') depth++;
    if (ch === ')') depth--;
    if (/\s/.test(ch) && depth === 0) {
      if (word) parts.push(word);
      word = '';
    } else word += ch;
  }
  if (word) parts.push(word);
  const [t, r = t, b = t, l = r] = parts;
  return [t, r, b, l];
}

const SIDE = { top: 0, right: 1, bottom: 2, left: 3 } as const;

interface IndexedRule {
  rule: CSSStyleRule;
  parts: Array<{ selector: string; spec: number; classes: string[]; tag: string | undefined }>;
}

/**
 * What the element a selector styles must carry, read from its last compound
 * (after the last combinator outside brackets and parentheses): the classes
 * and the tag named there, outside any `:not()` or `:is()`. An element
 * lacking one cannot match, so `matching` skips the costly `el.matches` for
 * it; a page lists over a hundred checkboxes, and the suite runs in parallel.
 */
function subject(selector: string): { classes: string[]; tag: string | undefined } {
  let depth = 0;
  let start = 0;
  for (let i = 0; i < selector.length; i++) {
    const ch = selector[i];
    if (ch === '(' || ch === '[') depth++;
    else if (ch === ')' || ch === ']') depth--;
    else if (depth === 0 && /[\s>+~]/.test(ch)) start = i + 1;
  }
  let last = '';
  depth = 0;
  for (const ch of selector.slice(start)) {
    if (ch === '(' || ch === '[') depth++;
    if (depth === 0) last += ch;
    if (ch === ')' || ch === ']') depth--;
  }
  const classes = (last.match(/\.[\w-]+/g) ?? []).map((c) => c.slice(1));
  const tag = /^[a-z][\w-]*/i.exec(last)?.[0].toUpperCase();
  return { classes, tag };
}

let index: { sheets: number; rules: IndexedRule[] } | null = null;

/** `styleRules()` with each selector split and weighed once, until a stylesheet is added or removed. */
function indexedRules(): IndexedRule[] {
  if (index?.sheets !== document.styleSheets.length) {
    const rules = styleRules().map((rule) => ({
      rule,
      parts: rule.selectorText.split(',').map((selector) => ({ selector, spec: specificity(selector), ...subject(selector) })),
    }));
    index = { sheets: document.styleSheets.length, rules };
  }
  return index.rules;
}

/**
 * The value of `property` that reaches `el` from the page's stylesheets, or
 * `undefined` when no rule declares it (the browser's own default then
 * applies). A `padding-*` or `margin-*` longhand is also read from its
 * shorthand.
 */
function declared(el: Element, property: string): string | undefined {
  const side = /^(padding|margin)-(top|right|bottom|left)$/.exec(property);
  let best: { spec: number; order: number; value: string } | undefined;
  for (const { rule, spec, order } of matching(el)) {
    let value = rule.style.getPropertyValue(property).trim();
    if (!value && side) {
      const short = rule.style.getPropertyValue(side[1]).trim();
      if (short) value = sides(short)[SIDE[side[2] as keyof typeof SIDE]];
    }
    if (!value) continue;
    if (!best || spec > best.spec || (spec === best.spec && order >= best.order)) best = { spec, order, value };
  }
  return best?.value;
}

const matched = new WeakMap<Element, { rules: IndexedRule[]; found: Array<{ rule: CSSStyleRule; spec: number; order: number }> }>();

/** The rules whose selector matches `el`, each with its highest matching specificity and its place in the page's order. */
function matching(el: Element): Array<{ rule: CSSStyleRule; spec: number; order: number }> {
  const rules = indexedRules();
  const cached = matched.get(el);
  if (cached?.rules === rules) return cached.found;
  const found: Array<{ rule: CSSStyleRule; spec: number; order: number }> = [];
  rules.forEach(({ rule, parts }, order) => {
    let spec = -1;
    for (const part of parts) {
      if (part.tag !== undefined && part.tag !== el.tagName) continue;
      if (!part.classes.every((c) => el.classList.contains(c))) continue;
      try {
        if (el.matches(part.selector)) spec = Math.max(spec, part.spec);
      } catch {
        // A selector jsdom cannot parse matches nothing here.
      }
    }
    if (spec >= 0) found.push({ rule, spec, order });
  });
  matched.set(el, { rules, found });
  return found;
}

/** A length in px: `var()` replaced from the tokens, then `calc()` worked out; `NaN` when it is not a length. */
function px(value: string | undefined, vars: Map<string, string>): number {
  if (value === undefined) return Number.NaN;
  let v = value;
  for (let i = 0; i < 10 && v.includes('var('); i++) {
    v = v.replace(/var\(\s*(--[a-z0-9-]+)\s*(?:,\s*([^()]*))?\)/gi, (_, name: string, fallback?: string) => vars.get(name) ?? fallback ?? 'NaN');
  }
  const expr = v.replace(/calc\(/g, '(').replace(/(\d)px/g, '$1');
  if (!/^[\d.\s*+\-/()]+$/.test(expr)) return Number.NaN;
  return Number(Function(`"use strict"; return (${expr});`)());
}

/**
 * Why a checkbox does not sit in line with its label, or nothing. The box
 * is inside its label row; the row centres the box on the text's line; the
 * box has no margin of its own, so it starts on the group heading's edge and
 * the gap to its text is the row's gap, two space units, in every list.
 */
function checkboxProblems(box: HTMLInputElement, vars: Map<string, string>): string[] {
  const name = `${box.closest('label')?.textContent?.trim() ?? '?'}`;
  const problems: string[] = [];
  const label = box.parentElement;
  if (label?.tagName !== 'LABEL') return [`${name}: the checkbox is not inside its label row`];
  if ((label.textContent ?? '').trim() === '') problems.push(`${name}: the label row has no text`);
  const display = declared(label, 'display');
  if (display !== 'flex' && display !== 'inline-flex' && display !== 'grid') problems.push(`${name}: the row is ${display ?? 'not'} flex`);
  const align = declared(label, 'align-items');
  if (align !== 'center') problems.push(`${name}: the row aligns its items ${align ?? 'by default'}, not to the centre`);
  const self = declared(box, 'align-self');
  if (self !== undefined && self !== 'auto' && self !== 'center') problems.push(`${name}: the box aligns itself ${self}`);
  for (const side of ['top', 'right', 'bottom', 'left']) {
    const margin = px(declared(box, `margin-${side}`), vars);
    if (margin !== 0) problems.push(`${name}: margin-${side} is ${Number.isNaN(margin) ? "the browser's own" : `${margin}px`}`);
  }
  const gap = px(declared(label, 'column-gap') ?? declared(label, 'gap')?.split(/\s+(?![^(]*\))/).pop(), vars);
  if (gap !== 8) problems.push(`${name}: the gap between box and text is ${Number.isNaN(gap) ? 'unset' : `${gap}px`}, not 8px`);
  return problems;
}

/**
 * The minimum inner padding of a button or text control (`ui-design.md`
 * § "Spacing and alignment"); a button that shows only an icon, named by its
 * `aria-label`, has no letter to keep off an edge and needs one space unit
 * on every side.
 */
const MIN_PAD = { top: 4, right: 8, bottom: 4, left: 8 } as const;
const MIN_PAD_ICON = { top: 4, right: 4, bottom: 4, left: 4 } as const;

/**
 * Why a button or text control's text could meet its edge, or nothing: each
 * side's padding is at least `MIN_PAD`, and a control at the standard 28 px
 * height keeps it (one line, its padding and its two borders fit in it).
 */
function paddingProblems(el: HTMLElement, vars: Map<string, string>): string[] {
  const shown =
    el instanceof HTMLSelectElement ? (el.selectedOptions[0]?.textContent ?? '') : (el.textContent || el.getAttribute('placeholder') || '');
  const iconOnly = el.tagName === 'BUTTON' && shown.trim() === '' && el.querySelector('svg') !== null;
  const name = `${el.tagName.toLowerCase()} "${(el.getAttribute('aria-label') ?? shown).trim().slice(0, 40)}"`;
  const min = iconOnly ? MIN_PAD_ICON : MIN_PAD;
  const problems: string[] = [];
  const pad = { top: 0, right: 0, bottom: 0, left: 0 };
  for (const side of ['top', 'right', 'bottom', 'left'] as const) {
    pad[side] = px(declared(el, `padding-${side}`), vars);
    if (!(pad[side] >= min[side])) {
      problems.push(`${name}: padding-${side} is ${Number.isNaN(pad[side]) ? 'unset' : `${pad[side]}px`}, under ${min[side]}px`);
    }
  }
  const minHeight = px(declared(el, 'min-height'), vars);
  if (minHeight === px('var(--size-button)', vars)) {
    const line = px(declared(el, 'line-height'), vars);
    const border = px('var(--border-width)', vars);
    const needs = line + pad.top + pad.bottom + 2 * border;
    if (!(needs <= minHeight)) {
      problems.push(`${name}: a line (${Number.isNaN(line) ? 'inherited' : `${line}px`}) and its padding need ${needs}px, over the ${minHeight}px height`);
    }
  }
  return problems;
}

/** Every button and text control in `root` (checkboxes and radios are not text controls). */
function controls(root: Element): HTMLElement[] {
  return Array.from(
    root.querySelectorAll<HTMLElement>('button, select, textarea, input:not([type=checkbox]):not([type=radio]):not([type=hidden])'),
  );
}

/** A Settings bridge with the catalogue the window's picker asks for. */
function settingsBridge(name: string): SettingsWindowBridge {
  return { ...fakeSettingsBridge(settingsFixture(name)), catalogue: async () => JSON.parse(JSON.stringify(CATALOGUE)) } as SettingsWindowBridge;
}

async function renderSettings(name: string): Promise<Rendered> {
  return renderComponent(SettingsWindow, { bridge: settingsBridge(name) }, (root) => root.querySelector('input[data-id]') !== null);
}

let mounted: Rendered[] = [];

afterEach(() => {
  for (const r of mounted) r.destroy();
  mounted = [];
});

// Each test mounts whole windows in jsdom and reads every rule against every
// control; the acceptance run starts this suite beside others, so the
// default five seconds is too little margin on a loaded machine.
describe('spacing and alignment', { timeout: 20_000 }, () => {
  it('the_style_reader_sees_what_a_rule_sets', async () => {
    // Positive and negative controls for the helpers above: a rule the
    // page adds is read and resolved, a shorthand with `var()` included, and
    // a row that aligns by baseline with the browser's margins is caught.
    const vars = await tokens();
    expect(px('calc(var(--space-unit) * 3)', vars)).toBe(12);
    expect(sides('0 calc(var(--space-unit) * 2)')).toEqual(['0', 'calc(var(--space-unit) * 2)', '0', 'calc(var(--space-unit) * 2)']);

    const style = document.createElement('style');
    style.textContent =
      '.probe-bad { display: flex; align-items: baseline; gap: 6px; }' +
      '.probe-good { display: flex; align-items: center; gap: calc(var(--space-unit) * 2); }' +
      '.probe-good input { margin: 0; }' +
      '.probe-btn { padding: 0 calc(var(--space-unit) * 3); }';
    document.head.appendChild(style);
    const host = document.createElement('div');
    host.innerHTML =
      '<label class="probe-bad"><input type="checkbox"> Bad</label>' +
      '<label class="probe-good"><input type="checkbox"> Good</label>' +
      '<button class="probe-btn">Tight</button>';
    document.body.appendChild(host);
    try {
      const [bad, good] = Array.from(host.querySelectorAll('input'));
      expect(checkboxProblems(bad, vars).join('\n')).toMatch(/baseline[\s\S]*margin-top is the browser's own[\s\S]*6px/);
      expect(checkboxProblems(good, vars)).toEqual([]);
      expect(paddingProblems(host.querySelector('button')!, vars)).toEqual([
        'button "Tight": padding-top is 0px, under 4px',
        'button "Tight": padding-bottom is 0px, under 4px',
      ]);
    } finally {
      host.remove();
      style.remove();
    }
  });

  it('currency_filter_checkboxes_align_with_their_labels', async () => {
    // The Settings window's blocked currencies: every entry's checkbox and
    // the "Blocked only" toggle sit in line with their labels.
    const vars = await tokens();
    const r = await renderSettings('blocked-currencies');
    mounted.push(r);
    const picker = r.root.querySelector('.picker');
    expect(picker, 'the Settings window shows the currency picker').not.toBeNull();
    const boxes = Array.from(picker!.querySelectorAll<HTMLInputElement>('input[type=checkbox]'));
    expect(boxes.length).toBeGreaterThan(5);
    const blockedOnly = boxes.find((b) => b.closest('label')?.textContent?.trim() === 'Blocked only');
    expect(blockedOnly, '"Blocked only" is a checkbox in its label').toBeDefined();
    expect(boxes.flatMap((b) => checkboxProblems(b, vars))).toEqual([]);

    // The same component in the craft window's Blocked for this craft.
    const alone = await renderComponent(
      CurrencyPicker,
      { groups: CATALOGUE, selected: [], tags: {}, onchange: () => {} },
      (root) => root.querySelector('input[data-id]') !== null,
    );
    mounted.push(alone);
    expect(Array.from(alone.root.querySelectorAll<HTMLInputElement>('input[type=checkbox]')).flatMap((b) => checkboxProblems(b, vars))).toEqual(
      [],
    );
  });

  it('target_checkboxes_align_with_their_labels', async () => {
    // The craft window's target choices: the same row as the currency
    // picker's, so the two lists' boxes start on their headings' edge.
    const vars = await tokens();
    const r = await renderView(fixtureView('craft-planning'));
    mounted.push(r);
    const boxes = Array.from(r.root.querySelectorAll<HTMLInputElement>('.choices input[type=checkbox]'));
    expect(boxes.length).toBeGreaterThan(5);
    expect(boxes.flatMap((b) => checkboxProblems(b, vars))).toEqual([]);
  });

  it('button_text_keeps_its_inner_padding', async () => {
    // Every button and text control in the main window (planning, guiding,
    // the trade panel) and the Settings window (the Trade section's
    // refresh icon and an install's buttons included) keeps the minimum
    // inner padding on all four sides, so no letter meets an edge when a
    // label wraps, and a standard control stays 28 px high.
    const vars = await tokens();
    const problems: string[] = [];
    let seen = 0;
    for (const name of ['craft-planning', 'craft-guiding', 'craft-done', 'trade-picking']) {
      const r = await renderView(fixtureView(name));
      mounted.push(r);
      for (const el of controls(r.root)) {
        seen++;
        problems.push(...paddingProblems(el, vars).map((p) => `${name}: ${p}`));
      }
    }
    for (const name of ['trade-league', 'install-failed', 'install-downloading']) {
      const r = await renderSettings(name);
      mounted.push(r);
      for (const el of controls(r.root)) {
        seen++;
        problems.push(...paddingProblems(el, vars).map((p) => `settings ${name}: ${p}`));
      }
    }
    expect(seen).toBeGreaterThan(20);
    expect(problems).toEqual([]);
  });
});
