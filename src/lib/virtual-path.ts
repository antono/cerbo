/**
 * Frontend port of `cerbo_core::vault::validate_virtual_path` (design D7):
 * the same rules and messages, so the dialogs can block a bad path before the
 * backend sees it. The backend stays the authority; its message wins when the
 * two disagree.
 */

/** Returns the broken rule, or null when `path` is valid. Empty means root. */
export function virtualPathError(path: string): string | null {
  if (path === '') return null;
  if (path.includes('\0')) return 'NUL byte';
  if (path.startsWith('/') || path.endsWith('/')) return 'leading or trailing slash';
  for (const segment of path.split('/')) {
    if (segment === '') return 'empty segment (double slash)';
    if (segment === '.' || segment === '..') return 'dot or dotdot segment';
  }
  return null;
}

export function isValidVirtualPath(path: string): boolean {
  return virtualPathError(path) === null;
}

/** The path as the backend stores it: trimmed, and null at the root. */
export function normalizeVirtualPath(path: string): string | null {
  const trimmed = path.trim();
  return trimmed === '' ? null : trimmed;
}

/** What a key does in the path field's open suggestion list. `highlighted` is -1 until the user moves to a suggestion with the arrow keys; `count` is the number of suggestions (> 0). */
export interface SuggestionStep {
  highlighted: number;
  /** Index of the suggestion to put in the field, or null. */
  pick: number | null;
  close: boolean;
  /** True when the key was consumed (the caller calls preventDefault). */
  handled: boolean;
}

export function suggestionKey(key: string, highlighted: number, count: number): SuggestionStep {
  switch (key) {
    case 'ArrowDown':
      return { highlighted: (highlighted + 1) % count, pick: null, close: false, handled: true };
    case 'ArrowUp':
      return { highlighted: highlighted <= 0 ? count - 1 : highlighted - 1, pick: null, close: false, handled: true };
    case 'Tab':
      return { highlighted: -1, pick: highlighted >= 0 ? highlighted : 0, close: false, handled: true };
    case 'Enter':
      return highlighted >= 0
        ? { highlighted: -1, pick: highlighted, close: false, handled: true }
        : { highlighted: -1, pick: null, close: true, handled: false };
    case 'Escape':
      return { highlighted: -1, pick: null, close: true, handled: true };
    default:
      return { highlighted, pick: null, close: false, handled: false };
  }
}
