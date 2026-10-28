// A paste into the main window is the second way to grab an item: the text
// comes from the paste event itself (`clipboardData`), which the webview
// fills because the player pressed Ctrl+V. The page never asks for the
// clipboard on its own, and a paste into an editable field is that field's,
// not a grab.

/** True when the paste lands in something the player is typing into. */
export function isEditable(target: EventTarget | null): boolean {
  if (!(target instanceof Element)) return false;
  if (target.closest('input, textarea, select')) return true;
  const host = target.closest('[contenteditable]');
  return host !== null && host.getAttribute('contenteditable') !== 'false';
}

/**
 * The text a paste carried, or null when the paste is not a grab (it went
 * into an editable field). A paste with no text at all gives `''`, which the
 * app refuses with a status line, so the player sees why nothing was grabbed.
 */
export function pastedText(event: Event): string | null {
  if (isEditable(event.target)) return null;
  const data = (event as ClipboardEvent).clipboardData;
  return data ? data.getData('text/plain') : '';
}
