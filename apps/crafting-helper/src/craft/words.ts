// The craft window's own words for the view's enums (`ui-design.md` §
// "UI-owned copy"): the guide's status, an outcome and where it came from,
// each said as a word so none is told by colour alone. The overlay uses the
// same words.
import type { GuideStatus, OutcomeSource, OutcomeWord } from '../ipc';

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

export const SOURCE_LABELS: Record<OutcomeSource, string> = {
  grab: 'from a grab',
  screen: 'from the screen',
};
