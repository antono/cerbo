import { describe, expect, it } from 'vitest';
import { SOURCE_FORMATS, defaultVaultName, formatBytes, formatMismatch, plural, progressFraction, progressLabel } from './import-vault';

describe('import-vault helpers', () => {
  it('lists Obsidian first, Tolaria second', () => {
    expect(SOURCE_FORMATS.map((f) => f.id)).toEqual(['obsidian', 'tolaria']);
  });

  it('reports a mismatch only when detection disagrees', () => {
    expect(formatMismatch('obsidian', { format: 'tolaria', reason: 'x' })).toBe('tolaria');
    expect(formatMismatch('obsidian', { format: 'obsidian', reason: 'x' })).toBeNull();
    expect(formatMismatch('obsidian', null)).toBeNull();
  });

  it('names a new vault after the folder', () => {
    expect(defaultVaultName('/home/a/Notes/')).toBe('Notes');
    expect(defaultVaultName('')).toBe('Imported vault');
  });

  it('formats sizes and counts', () => {
    expect(formatBytes(137)).toBe('137 B');
    expect(formatBytes(1536)).toBe('1.5 KB');
    expect(plural(1, 'page')).toBe('1 page');
    expect(plural(137, 'page')).toBe('137 pages');
  });

  it('maps progress to a monotonic 0..1 fraction', () => {
    const steps = [
      null,
      { phase: 'staging', done: 1, total: 137 },
      { phase: 'staging', done: 137, total: 137 },
      { phase: 'committing', done: 1, total: 137 },
      { phase: 'committing', done: 137, total: 137 },
      { phase: 'indexing', done: 0, total: 1 },
    ] as const;
    const f = steps.map((p) => progressFraction(p ? { ...p } : null));
    expect(f[0]).toBe(0);
    for (let i = 1; i < f.length; i++) expect(f[i]).toBeGreaterThan(f[i - 1]);
    expect(f[f.length - 1]).toBeLessThan(1);
    expect(progressLabel({ phase: 'staging', done: 3, total: 137 })).toBe('Preparing 3 of 137');
  });
});
