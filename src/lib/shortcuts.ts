/** Keyboard shortcuts shared by the app shell and the UI that advertises them. */

export const SEARCH_INPUT_ID = "search-input";

const isMac = typeof navigator !== "undefined" && /Mac|iPhone|iPad/.test(navigator.platform);

/** Label for the "focus search" shortcut on this platform. */
export const SEARCH_SHORTCUT_LABEL = isMac ? "⌘K" : "Ctrl K";

/** True for Ctrl+K (Windows/Linux) or ⌘K (macOS). */
export function isSearchShortcut(event: KeyboardEvent): boolean {
  return (event.ctrlKey || event.metaKey) && !event.altKey && event.key.toLowerCase() === "k";
}

export function focusSearchInput() {
  const input = document.getElementById(SEARCH_INPUT_ID) as HTMLInputElement | null;
  input?.focus();
  input?.select();
}
