// The words the plain UI shows for the contract's enum values. Display only:
// which value applies is the view's to say.

import type { QuestStatus, RewardCategory, RewardView, StepPhase, Worth } from './ipc';

export const STATUS_LABEL: Record<QuestStatus, string> = {
  locked: 'Locked',
  available: 'Available',
  in_progress: 'In progress',
  done: 'Done',
  skipped: 'Skipped',
  unknown: 'Unknown: tick it by hand',
};

/** The icon id of the brief for each status. */
export const STATUS_ICON: Record<QuestStatus, string> = {
  locked: 'status-locked',
  available: 'status-available',
  in_progress: 'status-in-progress',
  done: 'status-done',
  skipped: 'status-skipped',
  unknown: 'status-unknown',
};

export const WORTH_LABEL: Record<Worth, string> = {
  yes: 'Worth it',
  optional: 'Optional',
  no: 'Not worth it',
};

export const WORTH_ICON: Record<Worth, string> = {
  yes: 'worth-yes',
  optional: 'worth-optional',
  no: 'worth-no',
};

export const STEP_LABEL: Record<StepPhase, string> = {
  pending: 'To do',
  current: 'Next',
  done: 'Done',
};

export const STEP_ICON: Record<StepPhase, string> = {
  pending: 'status-available',
  current: 'status-in-progress',
  done: 'status-done',
};

// A quest row's reward column is narrow, so it shows a short label per
// reward; the full game text stays in the row's `title` and in the detail's
// reward chips. Keyed by the brief's reward icon id, which tells the
// resistances apart, with the category as the fallback for an id this table
// does not know.

export const REWARD_SHORT_BY_ICON: Record<string, string> = {
  'reward-passive': 'Passive',
  'reward-weapon-set': 'Weapon set',
  'reward-spirit': 'Spirit',
  'reward-res-fire': 'Fire res',
  'reward-res-cold': 'Cold res',
  'reward-res-lightning': 'Lightning res',
  'reward-res-chaos': 'Chaos res',
  'reward-res-all': 'All res',
  'reward-life': 'Life',
  'reward-mana': 'Mana',
  'reward-attribute': 'Attribute',
  'reward-charm': 'Charm',
  'reward-flask': 'Flask',
  'reward-defences': 'Defences',
  'reward-other': 'Stat',
  'reward-skill-gem': 'Skill gem',
  'reward-support-gem': 'Support gem',
  'reward-ascendancy': 'Ascendancy',
  'reward-item': 'Item',
  'reward-unlock': 'Unlock',
  'reward-story': 'Story',
};

export const REWARD_SHORT_BY_CATEGORY: Record<RewardCategory, string> = {
  passive_points: 'Passive',
  weapon_set_points: 'Weapon set',
  spirit: 'Spirit',
  resistance: 'Res',
  life: 'Life',
  mana: 'Mana',
  attribute: 'Attribute',
  charm: 'Charm',
  flask: 'Flask',
  defences: 'Defences',
  other_stat: 'Stat',
  skill_gem: 'Skill gem',
  support_gem: 'Support gem',
  ascendancy: 'Ascendancy',
  item: 'Item',
  unlock: 'Unlock',
  story: 'Story',
};

/** A reward's short label: by its icon id, else by its category, else its full text. */
export function rewardShortLabel(reward: RewardView): string {
  return REWARD_SHORT_BY_ICON[reward.icon] ?? REWARD_SHORT_BY_CATEGORY[reward.category] ?? reward.text;
}

/** A row's short reward labels, each once, in the view's order. */
export function rewardShortText(rewards: RewardView[]): string {
  return [...new Set(rewards.map(rewardShortLabel))].join(' · ');
}

/** A row's rewards in the game's full text, for the row's `title`. */
export function rewardText(rewards: RewardView[]): string {
  return rewards.map((r) => r.text).join(' · ');
}
