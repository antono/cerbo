import { describe, expect, it } from 'vitest';
import { isValidVirtualPath, normalizeVirtualPath, virtualPathError } from './virtual-path';

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
