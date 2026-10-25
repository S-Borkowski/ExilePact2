// The toast stack: each `qt:toast` is shown once, for about three seconds,
// at most three at a time below the panel. A fourth pushes out the oldest.
// The UI also raises its own Undo toast after Mark done and Skip in edit mode.

import type { QuestAction, Toast } from './ipc';

export const MAX_TOASTS = 3;
export const TOAST_MS = 3000;
export const UNDO_MS = 5000;

/** A toast as shown: the shell's `Toast`, or the UI's own Undo toast. */
export interface ShownToast {
  key: string;
  kind: Toast['kind'] | 'undo';
  text: string;
  icon: string;
  /** For an Undo toast: the action that reverses the one just sent. */
  undo?: { questId: string; action: QuestAction };
}

export function fromToast(toast: Toast): ShownToast {
  return { key: `qt-${toast.id}`, kind: toast.kind, text: toast.text, icon: toast.icon };
}

/** The stack with `toast` added at the bottom: a repeated key replaces its own entry, and the oldest beyond `max` go. */
export function pushToast(stack: readonly ShownToast[], toast: ShownToast, max = MAX_TOASTS): ShownToast[] {
  const rest = stack.filter((t) => t.key !== toast.key);
  return [...rest, toast].slice(-max);
}

export function dropToast(stack: readonly ShownToast[], key: string): ShownToast[] {
  return stack.filter((t) => t.key !== key);
}
