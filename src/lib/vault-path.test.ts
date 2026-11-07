import { describe, expect, it } from 'vitest';
import { splitVaultPath } from './vault-path';

describe('splitVaultPath', () => {
  it('keeps the last segment in the tail', () => {
    expect(splitVaultPath('/home/user/notes/work')).toEqual({ head: '/home/user/notes', tail: '/work' });
  });

  it('tells same-named vaults apart by head', () => {
    const a = splitVaultPath('/home/user/Notes');
    const b = splitVaultPath('/mnt/backup/137/Notes');
    expect(a.tail).toBe(b.tail);
    expect(a.head).not.toBe(b.head);
  });

  it('ignores trailing separators', () => {
    expect(splitVaultPath('/home/user/notes/')).toEqual({ head: '/home/user', tail: '/notes' });
  });

  it('handles a top-level folder and the root', () => {
    expect(splitVaultPath('/notes')).toEqual({ head: '', tail: '/notes' });
    expect(splitVaultPath('/')).toEqual({ head: '', tail: '/' });
  });

  it('splits Windows paths on backslashes', () => {
    expect(splitVaultPath('C:\\Users\\me\\Notes')).toEqual({ head: 'C:\\Users\\me', tail: '\\Notes' });
  });

  it('round-trips head + tail to the trimmed path', () => {
    const p = '/home/user/very/long/path/to/the/vault/folder';
    const { head, tail } = splitVaultPath(p);
    expect(head + tail).toBe(p);
  });
});
