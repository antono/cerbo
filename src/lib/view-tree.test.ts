import { describe, expect, it } from 'vitest';
import { flattenVisible } from './page-tree';
import {
  appendPage,
  buildViewTree,
  canMoveAt,
  hasRootPage,
  moveAt,
  removeAt,
  type ResolvedNode,
  type StoredNode,
} from './view-tree';

const page = (uuid: string, title = uuid): ResolvedNode => ({ kind: 'page', uuid, title, missing: false });

describe('buildViewTree', () => {
  const nodes: ResolvedNode[] = [
    { kind: 'folder', name: 'Cities', children: [page('a', 'Berlin'), page('b', 'Rome')] },
    page('a', 'Berlin'),
    { kind: 'page', uuid: 'gone', title: '', missing: true },
    { kind: 'query', name: 'Open', children: [page('c', 'Found')], total: 3, error: null },
    { kind: 'query', name: 'Broken', children: [], total: 0, error: 'boom' },
  ];
  const tree = buildViewTree(nodes);

  it('keeps the stored order and gives every folder a unique made-up path', () => {
    expect(tree.map((n) => (n.kind === 'folder' ? n.path : n.uuid))).toEqual(['~0', 'a', 'gone', '~3', '~4']);
  });

  it('lets the same page stand in two places with distinct row keys', () => {
    const rows = flattenVisible(tree, new Set());
    const keys = rows.filter((r) => r.kind === 'page' && r.uuid === 'a').map((r) => (r.kind === 'page' ? r.key : ''));
    expect(keys).toHaveLength(2);
    expect(new Set(keys).size).toBe(2);
  });

  it('marks a missing page and shows the query note', () => {
    const missing = tree[2];
    expect(missing).toMatchObject({ kind: 'page', title: 'Missing page', missing: true });
    expect(tree[3]).toMatchObject({ kind: 'folder', variant: 'query', info: '1 of 3', pageCount: 1 });
    expect(tree[4]).toMatchObject({ variant: 'query', info: 'error: boom' });
  });

  it('records where stored nodes sit, and none for pages a query found', () => {
    const cities = tree[0];
    expect(cities.kind === 'folder' && cities.children[1]).toMatchObject({ uuid: 'b', nodePath: [0, 1] });
    const open = tree[3];
    expect(open.kind === 'folder' && open.children[0]).toMatchObject({ uuid: 'c', nodePath: null });
    expect(open.kind === 'folder' && open.nodePath).toEqual([3]);
  });

  it('does not show the pages of a collapsed folder', () => {
    const rows = flattenVisible(tree, new Set(['~0']));
    expect(rows.some((r) => r.kind === 'page' && r.uuid === 'b')).toBe(false);
  });
});

describe('editing stored nodes', () => {
  const nodes: StoredNode[] = [
    { kind: 'folder', name: 'F', children: [{ kind: 'page', uuid: 'a' }, { kind: 'page', uuid: 'b' }] },
    { kind: 'page', uuid: 'c' },
  ];

  it('removes a nested node without touching the original', () => {
    const next = removeAt(nodes, [0, 0]);
    expect(next[0]).toEqual({ kind: 'folder', name: 'F', children: [{ kind: 'page', uuid: 'b' }] });
    expect(nodes[0]).toMatchObject({ children: [{ uuid: 'a' }, { uuid: 'b' }] });
  });

  it('moves among siblings and stops at either end', () => {
    expect(moveAt(nodes, [0, 0], 1)[0]).toMatchObject({ children: [{ uuid: 'b' }, { uuid: 'a' }] });
    expect(moveAt(nodes, [1], -1).map((n) => n.kind)).toEqual(['page', 'folder']);
    expect(canMoveAt(nodes, [0, 0], -1)).toBe(false);
    expect(canMoveAt(nodes, [1], 1)).toBe(false);
    expect(moveAt(nodes, [1], 1)).toBe(nodes);
  });

  it('ignores a path that names nothing', () => {
    expect(removeAt(nodes, [5])).toBe(nodes);
    expect(removeAt(nodes, [1, 0])).toBe(nodes);
  });

  it('appends a page reference at the root', () => {
    const next = appendPage(nodes, 'z');
    expect(next.at(-1)).toEqual({ kind: 'page', uuid: 'z' });
    expect(hasRootPage(next, 'z')).toBe(true);
    expect(hasRootPage(nodes, 'a')).toBe(false);
  });
});
