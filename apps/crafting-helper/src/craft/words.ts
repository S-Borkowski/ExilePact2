// The craft window's own words for the view's enums (`ui-design.md` §
// "UI-owned copy"): the guide's status and an outcome, each said as a word
// so neither is told by colour alone.
import type { GuideStatus, OutcomeWord } from '../ipc';

export const STATUS_LABELS: Record<GuideStatus, string> = {
  active: 'Crafting',
  stuck: 'Stuck',
  replan: 'Replan',
  done: 'Done',
};

export const OUTCOME_LABELS: Record<OutcomeWord, string> = {
  hit: 'Hit',
  miss: 'Miss',
  unexpected: 'Unexpected',
};
