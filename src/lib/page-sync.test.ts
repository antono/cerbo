import { describe, expect, it } from 'vitest';
import { buildPageContentDiff, decideExternalPageChange, logPageContentDiff, pageMdPathToSlug, pageChangeKey, shouldSkipExternalPageChange, shouldIgnoreUnchangedPageChange } from './page-sync';

describe('page-sync', () => {
  it('derives a page slug from a page.md path', () => {
    expect(pageMdPathToSlug('/vault', '/vault/notes/page.md')).toBe('notes');
  });

  it('reloads preview changes but prompts for dirty write mode changes', () => {
    expect(
      decideExternalPageChange({
        currentSlug: 'notes',
        changedSlug: 'notes',
        editorTab: 'preview',
        dirty: false,
      }),
    ).toBe('reload');

    expect(
      decideExternalPageChange({
        currentSlug: 'notes',
        changedSlug: 'notes',
        editorTab: 'write',
        dirty: true,
      }),
    ).toBe('prompt');
  });

  it('prompts in write mode even when the buffer is clean', () => {
    expect(
      decideExternalPageChange({
        currentSlug: 'notes',
        changedSlug: 'notes',
        editorTab: 'write',
        dirty: false,
      }),
    ).toBe('prompt');
  });

  it('skips the next matching external change event after overwrite', () => {
    const key = pageChangeKey('vault-1', '/vault/notes/page.md');
    expect(shouldSkipExternalPageChange(key, key)).toBe(true);
    expect(shouldSkipExternalPageChange(key, null)).toBe(false);
  });

  it('ignores filesystem events when disk content matches the editor', () => {
    expect(shouldIgnoreUnchangedPageChange('same', 'same')).toBe(true);
    expect(shouldIgnoreUnchangedPageChange('same', 'different')).toBe(false);
  });

  it('builds a simple diff for debug logging', () => {
    const diff = buildPageContentDiff('a\nb', 'a\nc');
    expect(diff).toContain('--- current');
    expect(diff).toContain('+++ disk');
    expect(diff).toContain('-b');
    expect(diff).toContain('+c');
  });

  it('emits a grouped console diff', () => {
    const groupCollapsed = console.groupCollapsed;
    const groupEnd = console.groupEnd;
    const log = console.log;
    const calls: Array<Array<unknown>> = [];
    console.groupCollapsed = ((...args: unknown[]) => calls.push(args)) as typeof console.groupCollapsed;
    console.groupEnd = (() => calls.push(['end'])) as typeof console.groupEnd;
    console.log = ((...args: unknown[]) => calls.push(args)) as typeof console.log;

    logPageContentDiff('label', 'a\nb', 'a\nc');

    console.groupCollapsed = groupCollapsed;
    console.groupEnd = groupEnd;
    console.log = log;

    expect(String(calls[0][0])).toContain('label');
    expect(calls.some((args) => String(args[0]).includes('--- current'))).toBe(true);
    expect(calls.some((args) => String(args[0]).includes('+++ disk'))).toBe(true);
    expect(calls.some((args) => String(args[0]).includes('-b'))).toBe(true);
    expect(calls.some((args) => String(args[0]).includes('+c'))).toBe(true);
  });
});
