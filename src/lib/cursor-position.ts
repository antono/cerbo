export interface SavedCursorPosition {
  line: number;
  column: number;
}

export interface RestoredCursorPosition {
  line: number;
  column: number;
  offset: number;
}

export function getCursorPositionFromOffset(value: string, offset: number): SavedCursorPosition {
  const safeOffset = Math.max(0, Math.min(offset, value.length));
  const before = value.slice(0, safeOffset);
  const line = before.split('\n').length;
  const lastNewline = before.lastIndexOf('\n');
  const column = safeOffset - (lastNewline + 1) + 1;
  return { line, column };
}

export function restoreCursorPosition(
  value: string,
  saved: SavedCursorPosition | null | undefined,
): RestoredCursorPosition {
  const lines = value.split('\n');
  const totalLines = Math.max(1, lines.length);
  const line = saved && saved.line >= 1 && saved.line <= totalLines ? saved.line : 2;
  const column = saved && saved.column >= 1 ? saved.column : 1;

  let offset = 0;
  const effectiveLine = Math.min(line, totalLines);
  for (let i = 1; i < effectiveLine; i += 1) {
    offset += lines[i - 1].length + 1;
  }

  return {
    line,
    column,
    offset: line > totalLines ? value.length : Math.min(offset + column - 1, value.length),
  };
}
