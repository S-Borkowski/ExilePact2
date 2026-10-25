// The words the plain UI shows for the contract's enum values. Display only:
// which value applies is the view's to say.

import type { QuestStatus, StepPhase, Worth } from './ipc';

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
