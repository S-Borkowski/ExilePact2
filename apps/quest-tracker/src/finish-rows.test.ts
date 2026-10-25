import { flushSync, mount, unmount } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import brief from '../../../docs/quest-tracker/ui-design-brief.md?raw';
import ActSection from './components/ActSection.svelte';
import QuestRowView from './components/QuestRow.svelte';
import { contractList } from './contract';
import type { ActView, QuestRow, QuestStatus, RewardView } from './ipc';
import { REWARD_CATEGORIES } from './ipc';
import { REWARD_SHORT_BY_CATEGORY, REWARD_SHORT_BY_ICON, rewardShortLabel, rewardText } from './labels';

// Finish review F1 and F2 (docs/quest-tracker/design/review-finish.md): the
// row's short reward labels and the act header's available count. Every view
// here is built in the test, never read from the fixtures, whose names and
// counts another item regenerates.

const mounted: Array<{ app: Record<string, unknown>; target: HTMLElement }> = [];

afterEach(() => {
  for (const { app, target } of mounted.splice(0)) {
    unmount(app);
    target.remove();
  }
});

function render<P extends Record<string, unknown>>(component: unknown, props: P): HTMLElement {
  const target = document.createElement('div');
  document.body.appendChild(target);
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const app = mount(component as any, { target, props }) as Record<string, unknown>;
  flushSync();
  mounted.push({ app, target });
  return target;
}

const send = () => {};

function row(id: string, status: QuestStatus, rewards: RewardView[] = []): QuestRow {
  return {
    id,
    name: `Quest ${id}`,
    act: 1,
    status,
    worth: 'yes',
    detection: 'auto',
    needs_tick: false,
    rewards,
    current_step: status === 'in_progress' ? 'Find the way on' : null,
  };
}

function act(quests: QuestRow[], has_available = quests.some((q) => q.status === 'available')): ActView {
  return {
    act: 1,
    title: 'Act 1',
    done: quests.filter((q) => q.status === 'done').length,
    total: quests.filter((q) => q.status !== 'skipped').length,
    expanded: true,
    has_available,
    quests,
  };
}

const SPIRIT: RewardView = { category: 'spirit', icon: 'reward-spirit', text: '+30 to Spirit' };
const PASSIVE: RewardView = { category: 'passive_points', icon: 'reward-passive', text: '2 Passive Skill Points' };
const COLD: RewardView = { category: 'resistance', icon: 'reward-res-cold', text: '+10% to Cold Resistance' };
const LIFE: RewardView = { category: 'life', icon: 'reward-life', text: '+20 to maximum Life' };

describe('finish review F1, F2', () => {
  it('row_reward_uses_short_labels', () => {
    // Every reward icon id of the brief, and every reward category, has a short label.
    const rewardIcons = contractList(brief, 'Icon ids').filter((id) => id.startsWith('reward-'));
    expect(rewardIcons.length).toBeGreaterThan(0);
    for (const id of rewardIcons) expect(REWARD_SHORT_BY_ICON[id], id).toBeTruthy();
    for (const category of REWARD_CATEGORIES) expect(REWARD_SHORT_BY_CATEGORY[category], category).toBeTruthy();

    // The finding's examples.
    expect(rewardShortLabel(SPIRIT)).toBe('Spirit');
    expect(rewardShortLabel(PASSIVE)).toBe('Passive');
    expect(rewardShortLabel(COLD)).toBe('Cold res');
    expect(rewardShortLabel(LIFE)).toBe('Life');
    // An icon id the table does not know falls back to the category's label.
    expect(rewardShortLabel({ category: 'spirit', icon: 'reward-new', text: '+30 to Spirit' })).toBe('Spirit');

    for (const edit of [false, true]) {
      const root = render(QuestRowView, { row: row('q1', 'available', [PASSIVE, COLD]), edit, send });
      const shown = root.querySelector('.rewards');
      expect(shown?.textContent).toBe('Passive · Cold res');
      expect(root.textContent).not.toContain('+10% to Cold Resistance');
    }

    // The same label twice reads once.
    const twice = render(QuestRowView, { row: row('q2', 'available', [PASSIVE, PASSIVE, SPIRIT]), edit: false, send });
    expect(twice.querySelector('.rewards')?.textContent).toBe('Passive · Spirit');
  });

  it('row_keeps_full_reward_text_in_title', () => {
    const rewards = [SPIRIT, COLD];
    expect(rewardText(rewards)).toBe('+30 to Spirit · +10% to Cold Resistance');
    for (const edit of [false, true]) {
      const root = render(QuestRowView, { row: row('q1', 'in_progress', rewards), edit, send });
      const shown = root.querySelector('.rewards');
      expect(shown?.textContent).toBe('Spirit · Cold res');
      expect(shown?.getAttribute('title')).toBe('+30 to Spirit · +10% to Cold Resistance');
    }
    // A quest with no rewards shows no title.
    const none = render(QuestRowView, { row: row('q3', 'available'), edit: false, send });
    expect(none.querySelector('.rewards')?.getAttribute('title') ?? '').toBe('');
  });

  it('act_header_counts_available_rows', () => {
    const header = (root: HTMLElement) => root.querySelector('.act-header')?.textContent?.replace(/\s+/g, ' ').trim() ?? '';

    const quests = [
      row('a', 'available'),
      row('b', 'in_progress'),
      row('c', 'available'),
      row('d', 'done'),
      row('e', 'available'),
      row('f', 'locked'),
      row('g', 'available'),
      row('h', 'skipped'),
    ];
    for (const edit of [false, true]) {
      const root = render(ActSection, { act: act(quests), edit, send });
      expect(root.querySelector('.available-count')?.textContent?.trim()).toBe('4 available ·');
      expect(header(root)).toContain('4 available · 1/7 done');
    }

    // One available: still counted.
    const one = render(ActSection, { act: act([row('a', 'available'), row('b', 'done')]), edit: false, send });
    expect(header(one)).toContain('1 available · 1/2 done');

    // Collapsed: the view carries no rows for it (qt-viewmodel sends `quests`
    // only when expanded), so the mark shows from `has_available` and no count.
    const collapsed = act([], true);
    collapsed.expanded = false;
    const folded = render(ActSection, { act: collapsed, edit: false, send });
    expect(folded.querySelector('.available')).not.toBeNull();
    expect(folded.querySelector('.available-count')).toBeNull();

    // None available: no count and no available mark.
    const none = render(ActSection, { act: act([row('a', 'done'), row('b', 'locked')]), edit: false, send });
    expect(none.querySelector('.available-count')).toBeNull();
    expect(header(none)).not.toContain('available');
    expect(header(none)).toContain('1/2 done');
  });
});
