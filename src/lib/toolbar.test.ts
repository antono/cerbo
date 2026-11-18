import { describe, expect, it } from 'vitest';
import { actionById } from './actions';
import { controlTitle, parentFolder, pathSegments, revealedAfterFocus, shouldOpenMenu } from './toolbar';

describe('toolbar path', () => {
  it('splits a virtual path into folder segments that reveal their folder', () => {
    expect(pathSegments('Projects')).toEqual([{ name: 'Projects', path: 'Projects' }]);
    expect(pathSegments('Projects/2026/Q4')).toEqual([
      { name: 'Projects', path: 'Projects' },
      { name: '2026', path: 'Projects/2026' },
      { name: 'Q4', path: 'Projects/2026/Q4' },
    ]);
  });

  it('has no segments at the root', () => {
    expect(pathSegments(null)).toEqual([]);
    expect(pathSegments(undefined)).toEqual([]);
    expect(pathSegments('')).toEqual([]);
  });

  it('finds the parent folder', () => {
    expect(parentFolder('Projects')).toBeNull();
    expect(parentFolder('Projects/2026')).toBe('Projects');
    expect(parentFolder('a/b/c')).toBe('a/b');
  });
});

describe('toolbar tooltips', () => {
  it('names the action and its hotkey', () => {
    expect(controlTitle('New page', actionById('page.new').hotkey, false)).toBe('New page (Ctrl+N)');
    expect(controlTitle('Hide page tree', actionById('panel.left').hotkey, false)).toBe('Hide page tree (Ctrl+\\)');
    expect(controlTitle('Show right panel', actionById('panel.right').hotkey, false)).toBe(
      'Show right panel (Ctrl+Shift+\\)',
    );
    expect(controlTitle('New page', actionById('page.new').hotkey, true)).toBe('New page (⌘+N)');
  });

  it('is the bare name without a hotkey', () => {
    expect(controlTitle('Copy link')).toBe('Copy link');
  });
});

describe('toolbar menu button', () => {
  it('opens when no menu just closed', () => {
    expect(shouldOpenMenu('more', null, 1000)).toBe(true);
  });

  it('stays closed when its own menu closed on this press', () => {
    expect(shouldOpenMenu('more', { kind: 'more', at: 990 }, 1000)).toBe(false);
  });

  it('opens when another menu closed, or long ago', () => {
    expect(shouldOpenMenu('more', { kind: 'page', at: 990 }, 1000)).toBe(true);
    expect(shouldOpenMenu('more', { kind: 'more', at: 137 }, 1000)).toBe(true);
  });
});

describe('revealed folder selection', () => {
  it('stays while its own row has focus', () => {
    expect(revealedAfterFocus('Projects', 'f:Projects')).toBe('Projects');
  });

  it('clears when focus moves to another row', () => {
    expect(revealedAfterFocus('Projects', 'f:Archive')).toBeNull();
    expect(revealedAfterFocus('Projects', 'p:page-137')).toBeNull();
    expect(revealedAfterFocus('Projects', 'f:Projects/2026')).toBeNull();
  });

  it('keeps it when focus is not on a row', () => {
    expect(revealedAfterFocus('Projects', null)).toBe('Projects');
    expect(revealedAfterFocus(null, 'f:Projects')).toBeNull();
  });
});
