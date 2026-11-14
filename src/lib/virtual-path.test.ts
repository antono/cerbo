import { describe, expect, it } from 'vitest';
import {
  isValidVirtualPath,
  normalizeVirtualPath,
  suggestionKey,
  virtualPathError,
} from './virtual-path';

// Mirrors `vault::validate_virtual_path` and its tests in core/src/vault.rs.
describe('virtualPathError', () => {
  it('accepts the root and plain segment paths', () => {
    expect(virtualPathError('')).toBeNull();
    expect(virtualPathError('notes')).toBeNull();
    expect(virtualPathError('notes/rust')).toBeNull();
    expect(virtualPathError('Заметки/2026 год')).toBeNull();
  });

  it('rejects leading or trailing slashes', () => {
    expect(virtualPathError('/notes')).toBe('leading or trailing slash');
    expect(virtualPathError('notes/')).toBe('leading or trailing slash');
  });

  it('rejects dot segments', () => {
    expect(virtualPathError('notes/./rust')).toBe('dot or dotdot segment');
    expect(virtualPathError('notes/../rust')).toBe('dot or dotdot segment');
  });

  it('rejects empty segments and NUL bytes', () => {
    expect(virtualPathError('notes//rust')).toBe('empty segment (double slash)');
    expect(virtualPathError('notes/\0rust')).toBe('NUL byte');
  });

  it('normalizes to null at the root', () => {
    expect(normalizeVirtualPath('  ')).toBeNull();
    expect(normalizeVirtualPath(' notes/rust ')).toBe('notes/rust');
    expect(isValidVirtualPath('a//b')).toBe(false);
  });
});

describe('suggestionKey', () => {
  it('Enter confirms the typed path when no suggestion was chosen', () => {
    expect(suggestionKey('Enter', -1, 3)).toEqual({ highlighted: -1, pick: null, close: true, handled: false });
  });

  it('Enter picks a suggestion reached with the arrow keys', () => {
    expect(suggestionKey('ArrowDown', -1, 3).highlighted).toBe(0);
    const enter = suggestionKey('Enter', 1, 3);
    expect(enter.pick).toBe(1);
    expect(enter.handled).toBe(true);
  });

  it('arrow keys wrap', () => {
    expect(suggestionKey('ArrowDown', 2, 3).highlighted).toBe(0);
    expect(suggestionKey('ArrowUp', -1, 3).highlighted).toBe(2);
    expect(suggestionKey('ArrowUp', 0, 3).highlighted).toBe(2);
  });

  it('Tab completes the highlighted or the first suggestion', () => {
    expect(suggestionKey('Tab', -1, 3).pick).toBe(0);
    expect(suggestionKey('Tab', 2, 3).pick).toBe(2);
  });

  it('Escape closes the list', () => {
    const esc = suggestionKey('Escape', 1, 3);
    expect(esc.close).toBe(true);
    expect(esc.handled).toBe(true);
  });
});
