import { describe, expect, it } from 'vitest';
import { getCursorPositionFromOffset, restoreCursorPosition } from './cursor-position';

describe('cursor-position', () => {
  it('derives line and column from offset', () => {
    expect(getCursorPositionFromOffset('# Title\nBody', 9)).toEqual({ line: 2, column: 2 });
  });

  it('restores a saved cursor position', () => {
    expect(restoreCursorPosition('# Title\nBody\nMore', { line: 3, column: 2 })).toEqual({
      line: 3,
      column: 2,
      offset: 14,
    });
  });

  it('falls back to line 2 for invalid saved positions', () => {
    expect(restoreCursorPosition('# Title\nBody\nMore', { line: 99, column: 4 })).toEqual({
      line: 2,
      column: 4,
      offset: 11,
    });
  });
});
