import { describe, expect, it } from 'vitest';
import { cursorKey, getCursorPositionFromOffset, restoreCursorPosition, scrollTopShowingCaret } from './cursor-position';

const page = '# Title\nBody\nMore';

describe('cursor-position', () => {
  it('derives line and column from offset', () => {
    expect(getCursorPositionFromOffset('# Title\nBody', 9)).toEqual({ line: 2, column: 2 });
  });

  it('restores a saved cursor position with its scroll', () => {
    expect(restoreCursorPosition(page, { line: 3, column: 2, scrollTop: 137 })).toEqual({
      line: 3,
      column: 2,
      offset: 14,
      scrollTop: 137,
    });
  });

  it('round-trips an offset through line and column', () => {
    const value = Array.from({ length: 200 }, (_, i) => `line ${i + 1}`).join('\n');
    const offset = value.indexOf('line 137') + 4;
    const saved = getCursorPositionFromOffset(value, offset);
    expect(saved).toEqual({ line: 137, column: 5 });
    expect(restoreCursorPosition(value, saved).offset).toBe(offset);
  });

  it('puts the caret at the start of line 2 when nothing is stored', () => {
    for (const saved of [null, undefined]) {
      expect(restoreCursorPosition(page, saved)).toEqual({ line: 2, column: 1, offset: 8, scrollTop: 0 });
    }
  });

  it('treats a broken stored line like no stored position', () => {
    expect(restoreCursorPosition(page, { line: 0, column: 4 })).toMatchObject({ line: 2, column: 1, offset: 8 });
    expect(restoreCursorPosition(page, { line: Number.NaN, column: 4 })).toMatchObject({ line: 2, offset: 8 });
  });

  it('puts the caret at the end of the page when the stored line is past the end', () => {
    const shortened = Array.from({ length: 40 }, (_, i) => `line ${i + 1}`).join('\n');
    expect(restoreCursorPosition(shortened, { line: 137, column: 4, scrollTop: 2000 })).toEqual({
      line: 40,
      column: 8,
      offset: shortened.length,
      scrollTop: 'end',
    });
  });

  it('goes to the end of a one-line page with nothing stored', () => {
    expect(restoreCursorPosition('# Title', null)).toEqual({ line: 1, column: 8, offset: 7, scrollTop: 'end' });
  });

  it('clamps the column to the end of a line that got shorter', () => {
    expect(restoreCursorPosition(page, { line: 2, column: 137 })).toEqual({
      line: 2,
      column: 5,
      offset: 12,
      scrollTop: 0,
    });
  });

  it('ignores a missing or invalid scroll offset', () => {
    expect(restoreCursorPosition(page, { line: 3, column: 1 }).scrollTop).toBe(0);
    expect(restoreCursorPosition(page, { line: 3, column: 1, scrollTop: -5 }).scrollTop).toBe(0);
    expect(restoreCursorPosition(page, { line: 3, column: 1, scrollTop: Number.POSITIVE_INFINITY }).scrollTop).toBe(0);
  });

  it('keys positions by vault and page', () => {
    expect(cursorKey('vault-a', 'page-1')).not.toBe(cursorKey('vault-b', 'page-1'));
  });

  describe('scrollTopShowingCaret', () => {
    // A 400 px view and 20 px rows; the caret row starts at `caretTop`.
    it('keeps a stored offset that shows the caret row', () => {
      expect(scrollTopShowingCaret(1370, 400, 1500, 20)).toBe(1370);
      expect(scrollTopShowingCaret(1370, 400, 1370, 20)).toBe(1370);
      expect(scrollTopShowingCaret(1370, 400, 1750, 20)).toBe(1370);
    });

    it('scrolls up to a caret row above the view', () => {
      expect(scrollTopShowingCaret(1370, 400, 137, 20)).toBe(137);
    });

    it('scrolls down just enough for a caret row below the view', () => {
      expect(scrollTopShowingCaret(0, 400, 1370, 20)).toBe(990);
      // Row only partly visible at the bottom edge.
      expect(scrollTopShowingCaret(1370, 400, 1760, 20)).toBe(1380);
    });

    it('never returns a negative offset', () => {
      expect(scrollTopShowingCaret(137, 400, -5, 20)).toBe(0);
    });
  });
});
