/**
 * Splits an absolute vault path for middle truncation in the vault selector.
 * `head` may be cut with an ellipsis by CSS; `tail` (the separator plus the
 * last segment, i.e. the vault folder name) always stays visible.
 * Both `/` and `\` count as separators; trailing separators are ignored.
 */
export function splitVaultPath(path: string): { head: string; tail: string } {
  const trimmed = path.replace(/[/\\]+$/, '');
  const cut = Math.max(trimmed.lastIndexOf('/'), trimmed.lastIndexOf('\\'));
  if (trimmed === '' || cut < 0) return { head: '', tail: path };
  return { head: trimmed.slice(0, cut), tail: trimmed.slice(cut) };
}
