import { describe, expect, it } from 'vitest';
import { buildPageTree, flattenVisible, type TreePage, type TreeRow } from './page-tree';
import { dropPlacement, dropZone, passedThreshold, rootPlacement } from './tree-drag';

function page(title: string, virtualPath: string | null = null): TreePage {
  return { uuid: title.toLowerCase(), title, virtualPath };
}

const pages = [page('Alpha'), page('Beta'), page('Own', 'notes'), page('Deep', 'notes/rust')];
const tree = buildPageTree(pages, { pinned: ['archive'], order: {} });
const rows = flattenVisible(tree, new Set());
const row = (label: string): TreeRow =>
  rows.find((r) => (r.kind === 'folder' ? r.path : r.title) === label) ?? (() => { throw new Error(label); })();

describe('dropZone (design D7)', () => {
  // A 20 px row from y = 100.
  const at = (r: TreeRow, y: number) => dropZone(r, 100, 20, y);

  it('splits a page row in halves', () => {
    expect(at(row('Alpha'), 101)).toBe('before');
    expect(at(row('Alpha'), 109)).toBe('before');
    expect(at(row('Alpha'), 111)).toBe('after');
    expect(at(row('Alpha'), 119)).toBe('after');
  });

  it('gives a collapsed or empty folder before / inside / after', () => {
    const archive = row('archive'); // pinned, empty
    expect(at(archive, 103)).toBe('before');
    expect(at(archive, 110)).toBe('inside-last');
    expect(at(archive, 117)).toBe('after');
    const collapsed = { ...row('notes'), expanded: false };
    expect(at(collapsed, 117)).toBe('after');
  });

  it('puts the bottom of an expanded folder with children inside, first', () => {
    expect(at(row('notes'), 117)).toBe('inside-first');
  });
});

describe('dropPlacement', () => {
  const alpha = { kind: 'page' as const, uuid: 'alpha', path: null };

  it('before and after a page in another folder move and place the row', () => {
    const p = dropPlacement(tree, alpha, row('Own'), 'after');
    expect(p).toEqual({ move: { kind: 'page', uuid: 'alpha', to: 'notes' }, parent: 'notes', order: ['rust/', 'own', 'alpha'] });
    expect(dropPlacement(tree, alpha, row('Own'), 'before')?.order).toEqual(['rust/', 'alpha', 'own']);
  });

  it('inside a folder puts the row last, or first', () => {
    expect(dropPlacement(tree, alpha, row('notes'), 'inside-last')?.order).toEqual(['rust/', 'own', 'alpha']);
    expect(dropPlacement(tree, alpha, row('notes'), 'inside-first')?.order).toEqual(['alpha', 'rust/', 'own']);
  });

  it('refuses a folder over its own subtree', () => {
    const notes = { kind: 'folder' as const, path: 'notes' };
    expect(dropPlacement(tree, notes, row('notes'), 'inside-last')).toBeNull();
    expect(dropPlacement(tree, notes, row('notes/rust'), 'before')).toBeNull();
    expect(dropPlacement(tree, notes, row('Deep'), 'after')).toBeNull();
  });

  it('a folder dropped inside another folder is renamed under it', () => {
    const p = dropPlacement(tree, { kind: 'folder', path: 'notes/rust' }, row('archive'), 'inside-last');
    expect(p?.move).toEqual({ kind: 'folder', from: 'notes/rust', to: 'archive/rust' });
  });

  it('dropping next to itself changes nothing', () => {
    expect(dropPlacement(tree, alpha, row('Alpha'), 'before')).toBeNull();
    expect(dropPlacement(tree, alpha, row('Alpha'), 'after')).toBeNull();
    expect(dropPlacement(tree, alpha, row('Beta'), 'before')).toBeNull();
  });

  it('a page row has no inside', () => {
    expect(dropPlacement(tree, alpha, row('Beta'), 'inside-last')).toBeNull();
  });

  it('the root zone puts the row last at the root', () => {
    const p = rootPlacement(tree, { kind: 'page', uuid: 'own', path: 'notes' });
    expect(p?.move).toEqual({ kind: 'page', uuid: 'own', to: null });
    expect(p?.order.at(-1)).toBe('own');
  });
});

describe('passedThreshold', () => {
  it('needs more than 4 px of travel', () => {
    expect(passedThreshold(3, 2)).toBe(false);
    expect(passedThreshold(4, 0)).toBe(false);
    expect(passedThreshold(4, 1)).toBe(true);
  });
});
