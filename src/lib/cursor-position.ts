// Caret and scroll of the write view, saved when an editing session on a page
// ends and restored when the page is edited again (spec
// `editor-position-restore`). Lines and columns are 1-based.

export interface SavedCursorPosition {
  line: number;
  column: number;
  /** Scroll offset of the write view; absent in positions from before it was stored. */
  scrollTop?: number;
}

export interface RestoredCursorPosition {
  line: number;
  column: number;
  offset: number;
  /** Where to scroll the write view: a pixel offset, or the end of the page. */
  scrollTop: number | 'end';
}

export function getCursorPositionFromOffset(value: string, offset: number): SavedCursorPosition {
  const safeOffset = Math.max(0, Math.min(offset, value.length));
  const before = value.slice(0, safeOffset);
  const line = before.split('\n').length;
  const lastNewline = before.lastIndexOf('\n');
  const column = safeOffset - (lastNewline + 1) + 1;
  return { line, column };
}

/**
 * Where the caret goes when entering write mode. A stored line that still
 * exists is used with its column (clamped to the line) and scroll; without a
 * usable position the caret goes to the start of line 2, the first body line
 * after the title; a stored line past the end puts it at the end of the page.
 */
export function restoreCursorPosition(
  value: string,
  saved: SavedCursorPosition | null | undefined,
): RestoredCursorPosition {
  const lines = value.split('\n');
  const totalLines = lines.length;
  const lineStart = (line: number) => {
    let offset = 0;
    for (let i = 1; i < line; i += 1) offset += lines[i - 1].length + 1;
    return offset;
  };
  const endOfPage = (): RestoredCursorPosition => ({
    line: totalLines,
    column: lines[totalLines - 1].length + 1,
    offset: value.length,
    scrollTop: 'end',
  });

  const valid = saved && Number.isInteger(saved.line) && saved.line >= 1;
  if (valid && saved.line > totalLines) return endOfPage();
  if (!valid) {
    if (totalLines < 2) return endOfPage();
    return { line: 2, column: 1, offset: lineStart(2), scrollTop: 0 };
  }

  const lineLength = lines[saved.line - 1].length;
  const column = Math.min(Math.max(1, Math.floor(saved.column) || 1), lineLength + 1);
  const scrollTop = typeof saved.scrollTop === 'number' && Number.isFinite(saved.scrollTop) ? Math.max(0, saved.scrollTop) : 0;
  return { line: saved.line, column, offset: lineStart(saved.line) + column - 1, scrollTop };
}

/** Key of a page's position: one per vault and page. */
export function cursorKey(vaultId: string, uuid: string): string {
  return `${vaultId}/${uuid}`;
}

/**
 * Scroll offset that shows the caret's row: `scrollTop` when the row
 * (`caretTop` to `caretTop + rowHeight`, in scroll coordinates) is already in
 * the `clientHeight` tall view, else the nearest offset that brings it in. A
 * stored offset can miss the caret after the window or font size changed, or
 * lines were added above it since the save.
 */
export function scrollTopShowingCaret(scrollTop: number, clientHeight: number, caretTop: number, rowHeight: number): number {
  if (caretTop < scrollTop) return Math.max(0, caretTop);
  if (caretTop + rowHeight > scrollTop + clientHeight) return Math.max(0, caretTop + rowHeight - clientHeight);
  return scrollTop;
}
