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
