import { describe, expect, it } from 'vitest';
import {
  ancestorFolders,
  buildPageTree,
  carryCollapsed,
  flattenVisible,
  folderPaths,
  parentRowIndex,
  pruneCollapsed,
  revealPath,
  treeKeyAction,
  visiblePageUuids,
  childrenOf,
  entryKey,
  itemOf,
  moveBy,
  placement,
  type TreeLayout,
  type TreeNode,
  type TreePage,
} from './page-tree';

function page(title: string, virtualPath: string | null = null, uuid = title.toLowerCase()): TreePage {
  return { uuid, title, virtualPath };
}

const vault: TreePage[] = [
  page('Ownership', 'notes/rust'),
  page('Index', 'notes'),
  page('Home'),
  page('Zebra'),
  page('Book', 'notebook'),
  page('Async', 'notes/rust/async'),
];

/** `depth:kind:name` per row, for compact assertions. */
function shape(rows: ReturnType<typeof flattenVisible>): string[] {
  return rows.map((r) => `${r.depth}:${r.kind}:${r.kind === 'folder' ? r.name : r.title}`);
}

describe('buildPageTree', () => {
  it('nests pages under their path segments, folders first, sorted by name', () => {
    const rows = flattenVisible(buildPageTree(vault), new Set());
    expect(shape(rows)).toEqual([
      '0:folder:notebook',
      '1:page:Book',
      '0:folder:notes',
      '1:folder:rust',
      '2:folder:async',
      '3:page:Async',
      '2:page:Ownership',
      '1:page:Index',
      '0:page:Home',
      '0:page:Zebra',
    ]);
  });

  it('renders a flat vault as plain page rows sorted by title', () => {
    const rows = flattenVisible(buildPageTree([page('b'), page('A'), page('c')]), new Set());
    expect(shape(rows)).toEqual(['0:page:A', '0:page:b', '0:page:c']);
  });

  it('keeps notebook apart from notes', () => {
    const tree = buildPageTree(vault);
    expect(tree.map((n) => (n.kind === 'folder' ? n.path : n.title))).toEqual(['notebook', 'notes', 'Home', 'Zebra']);
  });

  it('treats an empty path as the root', () => {
    const rows = flattenVisible(buildPageTree([{ uuid: 'x', title: 'X', virtualPath: '' }]), new Set());
    expect(shape(rows)).toEqual(['0:page:X']);
  });

  it('builds intermediate folders that no page sits in directly', () => {
    const rows = flattenVisible(buildPageTree([page('Deep', 'a/b/c')]), new Set());
    expect(shape(rows)).toEqual(['0:folder:a', '1:folder:b', '2:folder:c', '3:page:Deep']);
  });
});

describe('flattenVisible', () => {
  it('omits everything inside a collapsed folder', () => {
    const rows = flattenVisible(buildPageTree(vault), new Set(['notes']));
    expect(shape(rows)).toEqual(['0:folder:notebook', '1:page:Book', '0:folder:notes', '0:page:Home', '0:page:Zebra']);
    expect(rows[2]).toMatchObject({ kind: 'folder', expanded: false });
  });

  it('collapsing a nested folder keeps its siblings', () => {
    const rows = flattenVisible(buildPageTree(vault), new Set(['notes/rust']));
    expect(shape(rows)).toContain('1:folder:rust');
    expect(shape(rows)).toContain('1:page:Index');
    expect(shape(rows)).not.toContain('2:page:Ownership');
  });
});

describe('folder helpers', () => {
  it('lists distinct folder paths including intermediate ones, sorted', () => {
    expect(folderPaths(vault)).toEqual(['notebook', 'notes', 'notes/rust', 'notes/rust/async']);
    expect(folderPaths([page('A'), page('B')])).toEqual([]);
  });

  it('expands a path into its ancestors', () => {
    expect(ancestorFolders('a/b/c')).toEqual(['a', 'a/b', 'a/b/c']);
    expect(ancestorFolders(null)).toEqual([]);
    expect(ancestorFolders('')).toEqual([]);
  });

  it('prunes collapsed paths that no longer exist', () => {
    expect(pruneCollapsed(['notes', 'drafts', 'notes/rust', 'notes'], vault)).toEqual(['notes', 'notes/rust']);
  });

  it('reveals a page by expanding every folder on its path', () => {
    expect(revealPath(['notes', 'notes/rust', 'notebook'], 'notes/rust/async').sort()).toEqual(['notebook']);
    expect(revealPath(['notes'], null)).toEqual(['notes']);
  });

  it('carries collapsed state over a folder rename', () => {
    expect(carryCollapsed(['notes', 'notes/rust', 'notebook'], 'notes', 'archive').sort()).toEqual([
      'archive',
      'archive/rust',
      'notebook',
    ]);
    expect(carryCollapsed(['notes/rust'], 'notes/rust', 'notes/lang/rust')).toEqual(['notes/lang/rust']);
  });
});

describe('treeKeyAction', () => {
  const rows = flattenVisible(buildPageTree(vault), new Set(['notes/rust']));
  // 0 notebook, 1 Book, 2 notes, 3 rust (collapsed), 4 Index, 5 Home, 6 Zebra
  const at = (name: string) => rows.findIndex((r) => (r.kind === 'folder' ? r.name : r.title) === name);

  it('steps over visible rows and wraps', () => {
    expect(treeKeyAction(rows, at('Zebra'), 'next')).toEqual({ kind: 'focus', index: 0 });
    expect(treeKeyAction(rows, 0, 'prev')).toEqual({ kind: 'focus', index: rows.length - 1 });
    expect(treeKeyAction(rows, at('rust'), 'next')).toEqual({ kind: 'focus', index: at('Index') });
    expect(treeKeyAction(rows, -1, 'next')).toEqual({ kind: 'focus', index: 0 });
    expect(treeKeyAction(rows, -1, 'prev')).toEqual({ kind: 'focus', index: rows.length - 1 });
    expect(treeKeyAction([], 0, 'next')).toBeNull();
  });

  it('left collapses an expanded folder, else goes to the parent', () => {
    expect(treeKeyAction(rows, at('notes'), 'left')).toEqual({ kind: 'collapse', path: 'notes' });
    expect(treeKeyAction(rows, at('rust'), 'left')).toEqual({ kind: 'focus', index: at('notes') });
    expect(treeKeyAction(rows, at('Index'), 'left')).toEqual({ kind: 'focus', index: at('notes') });
    expect(treeKeyAction(rows, at('Home'), 'left')).toBeNull();
  });

  it('right expands a collapsed folder, else enters it', () => {
    expect(treeKeyAction(rows, at('rust'), 'right')).toEqual({ kind: 'expand', path: 'notes/rust' });
    expect(treeKeyAction(rows, at('notes'), 'right')).toEqual({ kind: 'focus', index: at('rust') });
    expect(treeKeyAction(rows, at('Home'), 'right')).toBeNull();
  });

  it('enter toggles a folder or opens a page; space only toggles', () => {
    expect(treeKeyAction(rows, at('notes'), 'enter')).toEqual({ kind: 'toggle', path: 'notes' });
    expect(treeKeyAction(rows, at('Home'), 'enter')).toEqual({ kind: 'open', uuid: 'home' });
    expect(treeKeyAction(rows, at('Home'), 'space')).toBeNull();
    expect(treeKeyAction(rows, at('rust'), 'space')).toEqual({ kind: 'toggle', path: 'notes/rust' });
  });

  it('finds the parent row across nested folders', () => {
    const all = flattenVisible(buildPageTree(vault), new Set());
    const i = all.findIndex((r) => r.kind === 'page' && r.title === 'Async');
    expect(all[parentRowIndex(all, i)]).toMatchObject({ kind: 'folder', path: 'notes/rust/async' });
    expect(parentRowIndex(all, 0)).toBe(-1);
  });

  it('lists visible page uuids in sidebar order', () => {
    expect(visiblePageUuids(rows)).toEqual(['book', 'index', 'home', 'zebra']);
  });
});


// ── Layout: order and pinned folders (add-sidebar-ordering D2) ──────────────
// The same cases as `core/src/tree.rs`, so both sides read a layout alike.

function layout(order: Record<string, string[]> = {}, pinned: string[] = []): TreeLayout {
  return { pinned, order };
}

/** Depth-first labels: folders as `name/`, pages by title, two spaces per level. */
function labels(nodes: TreeNode[], depth = 0): string[] {
  return nodes.flatMap((n) =>
    n.kind === 'folder'
      ? [`${'  '.repeat(depth)}${n.name}/`, ...labels(n.children, depth + 1)]
      : [`${'  '.repeat(depth)}${n.title}`],
  );
}

describe('buildPageTree with a layout', () => {
  it('without a layout, folders come first, then titles case-insensitively', () => {
    const tree = buildPageTree([page('beta'), page('Alpha'), page('Own', 'notes/rust')]);
    expect(labels(tree)).toEqual(['notes/', '  rust/', '    Own', 'Alpha', 'beta']);
  });

  it('puts listed children first, then the rest in default order', () => {
    const pages = [page('Alpha', 'notes'), page('Beta', 'notes'), page('Own', 'notes/rust')];
    const tree = buildPageTree(pages, layout({ notes: ['beta'] }));
    expect(labels(tree)).toEqual(['notes/', '  Beta', '  rust/', '    Own', '  Alpha']);
  });

  it('skips stale and duplicate entries', () => {
    const tree = buildPageTree([page('Home'), page('Other')], layout({ '': ['gone', 'other', 'gone/', 'other', 'home'] }));
    expect(labels(tree)).toEqual(['Other', 'Home']);
  });

  it('renders pinned folders with their ancestors even when empty', () => {
    const tree = buildPageTree([page('Home')], layout({}, ['projects/2026']));
    expect(labels(tree)).toEqual(['projects/', '  2026/', 'Home']);
    const projects = tree[0];
    expect(projects.kind === 'folder' && projects.pinned).toBe(false);
    const year = projects.kind === 'folder' ? projects.children[0] : null;
    expect(year?.kind === 'folder' && year.pinned).toBe(true);
  });

  it('can list a page before a folder', () => {
    const tree = buildPageTree([page('Home'), page('Own', 'notes')], layout({ '': ['home', 'notes/'] }));
    expect(labels(tree)).toEqual(['Home', 'notes/', '  Own']);
  });

  it('counts pages at or under each folder', () => {
    const tree = buildPageTree(vault, layout({}, ['notes/empty']));
    const notes = tree.find((n) => n.kind === 'folder' && n.path === 'notes');
    expect(notes?.kind === 'folder' && notes.pageCount).toBe(3);
    const empty = childrenOf(tree, 'notes')?.find((n) => n.kind === 'folder' && n.name === 'empty');
    expect(empty?.kind === 'folder' && empty.pageCount).toBe(0);
  });

  it('includes pinned folders in folder paths and keeps them through pruning', () => {
    const l = layout({}, ['inbox']);
    expect(folderPaths([page('Home')], l)).toEqual(['inbox']);
    expect(pruneCollapsed(['inbox', 'gone'], [page('Home')], l)).toEqual(['inbox']);
  });
});

// ── Placement (add-sidebar-ordering D6) ─────────────────────────────────────

describe('placement', () => {
  const pages = [page('Alpha'), page('Beta'), page('Gamma'), page('Own', 'notes'), page('Deep', 'notes/rust')];
  const tree = buildPageTree(pages, layout({}, ['archive']));
  const root = () => childrenOf(tree, '')!.map(entryKey);

  it('reorders within a folder without a move', () => {
    expect(root()).toEqual(['archive/', 'notes/', 'alpha', 'beta', 'gamma']);
    const p = placement(tree, { kind: 'page', uuid: 'gamma', path: null }, '', 2);
    expect(p).toEqual({ move: null, parent: '', order: ['archive/', 'notes/', 'gamma', 'alpha', 'beta'] });
  });

  it('moving down accounts for the row leaving its old place', () => {
    const p = placement(tree, { kind: 'page', uuid: 'alpha', path: null }, '', 4);
    expect(p?.order).toEqual(['archive/', 'notes/', 'beta', 'alpha', 'gamma']);
  });

  it('returns null when the row stays where it is', () => {
    expect(placement(tree, { kind: 'page', uuid: 'alpha', path: null }, '', 2)).toBeNull();
    expect(placement(tree, { kind: 'page', uuid: 'alpha', path: null }, '', 3)).toBeNull();
  });

  it('moves a page to another folder and places it there', () => {
    const p = placement(tree, { kind: 'page', uuid: 'alpha', path: null }, 'notes', 2);
    expect(p).toEqual({ move: { kind: 'page', uuid: 'alpha', to: 'notes' }, parent: 'notes', order: ['rust/', 'own', 'alpha'] });
  });

  it('moves a page to the root with a null path', () => {
    const p = placement(tree, { kind: 'page', uuid: 'own', path: 'notes' }, '', 0);
    expect(p?.move).toEqual({ kind: 'page', uuid: 'own', to: null });
    expect(p?.order[0]).toBe('own');
  });

  it('moves a folder by renaming it under the new parent', () => {
    const p = placement(tree, { kind: 'folder', path: 'notes/rust' }, 'archive', 0);
    expect(p).toEqual({ move: { kind: 'folder', from: 'notes/rust', to: 'archive/rust' }, parent: 'archive', order: ['rust/'] });
  });

  it('refuses a folder inside itself or its descendant', () => {
    expect(placement(tree, { kind: 'folder', path: 'notes' }, 'notes', 0)).toBeNull();
    expect(placement(tree, { kind: 'folder', path: 'notes' }, 'notes/rust', 0)).toBeNull();
  });

  it('keeps notebook apart from notes when refusing', () => {
    const t = buildPageTree([page('A', 'notes'), page('B', 'notebook')]);
    expect(placement(t, { kind: 'folder', path: 'notes' }, 'notebook', 0)?.move).toEqual({
      kind: 'folder',
      from: 'notes',
      to: 'notebook/notes',
    });
  });

  it('moveBy swaps with a neighbour and stops at the ends', () => {
    const rows = flattenVisible(tree, new Set());
    const beta = rows.find((r) => r.kind === 'page' && r.uuid === 'beta')!;
    expect(moveBy(tree, beta, -1)?.order).toEqual(['archive/', 'notes/', 'beta', 'alpha', 'gamma']);
    expect(moveBy(tree, beta, 1)?.order).toEqual(['archive/', 'notes/', 'alpha', 'gamma', 'beta']);
    const archive = rows.find((r) => r.kind === 'folder' && r.path === 'archive')!;
    expect(moveBy(tree, archive, -1)).toBeNull();
    const gamma = rows.find((r) => r.kind === 'page' && r.uuid === 'gamma')!;
    expect(moveBy(tree, gamma, 1)).toBeNull();
    expect(itemOf(gamma)).toEqual({ kind: 'page', uuid: 'gamma', path: null });
  });
});
