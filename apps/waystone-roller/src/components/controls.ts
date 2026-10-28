// Keeps a radio group showing the view's value after the player picks
// another one: the pick is sent to the shell, and the group changes when the
// shell's next view says so. Without this, a refused pick would stay checked
// on screen, since the view's own value did not change.
export function restoreGroup(input: HTMLInputElement, current: string): void {
  const scope = input.closest('[role="radiogroup"]') ?? input.form ?? document;
  for (const other of scope.querySelectorAll<HTMLInputElement>('input[type="radio"]')) {
    if (other.name === input.name) other.checked = other.value === current;
  }
}
