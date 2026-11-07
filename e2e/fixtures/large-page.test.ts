import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import { LARGE_PAGE_BYTES, LARGE_PAGE_TITLE, LARGE_PAGE_UUID, largePageMarkdown, writeLargePage } from './large-page';

describe('large page fixture', () => {
  it('is exactly 137 KB of ASCII starting with the title', () => {
    const md = largePageMarkdown();
    expect(md.length).toBe(LARGE_PAGE_BYTES);
    expect(new TextEncoder().encode(md).length).toBe(LARGE_PAGE_BYTES);
    expect(md.startsWith(`# ${LARGE_PAGE_TITLE}\n`)).toBe(true);
    expect(md.endsWith('\n')).toBe(true);
  });

  it('is deterministic', () => {
    expect(largePageMarkdown()).toBe(largePageMarkdown());
  });

  it('writes a page object into a vault', () => {
    const vault = mkdtempSync(join(tmpdir(), 'cerbo-large-page-'));
    try {
      const dir = writeLargePage(vault);
      expect(dir).toBe(join(vault, '.cerbo', 'objects', LARGE_PAGE_UUID));
      expect(readFileSync(join(dir, 'page.md'), 'utf8')).toBe(largePageMarkdown());
      expect(readFileSync(join(dir, 'meta.ttl'), 'utf8')).toContain(`schema:name "${LARGE_PAGE_TITLE}"`);
    } finally {
      rmSync(vault, { recursive: true, force: true });
    }
  });
});
