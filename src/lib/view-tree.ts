/**
 * Views in the sidebar (spec `vault-views`, `desktop-view-switcher`; design
 * D8). A view is an alternative tree over the same pages. `buildViewTree` turns
 * what `view_get` returns into the `TreeNode`s the sidebar already renders, and
 * the functions below edit the stored nodes that `view_update` takes back.
 * Pure, so it is tested without a DOM or Tauri.
 */
import type { FolderNode, PageNode, TreeNode } from './page-tree';

export interface ViewMeta {
  uuid: string;
  title: string;
}

/** A node as `view.toml` and `view_update` hold it. */
export type StoredNode =
  | { kind: 'folder'; name: string; children?: StoredNode[] }
  | { kind: 'page'; uuid: string }
  | { kind: 'query'; name: string; sparql: string };

/** A node as `view_get` returns it, pages looked up and queries run. */
export type ResolvedNode =
  | { kind: 'folder'; name: string; children: ResolvedNode[] }
  | { kind: 'page'; uuid: string; title: string; missing: boolean }
  | { kind: 'query'; name: string; children: ResolvedNode[]; total: number; error: string | null };

export interface ViewData {
  uuid: string;
  title: string;
  nodes: ResolvedNode[];
}

/** Where a view's own tree starts; a path never collides with a vault folder path. */
const ROOT = '~';

function countPages(nodes: TreeNode[]): number {
  return nodes.reduce((n, node) => n + (node.kind === 'page' ? 1 : node.pageCount), 0);
}

/**
 * The sidebar tree of a view. Folder paths are made up (`~0/1`, the indexes of
 * the stored node), so they are unique and never match a vault folder. A page
 * row's `key` is unique too: the same page may stand in several places. Nodes a
 * query produced are not stored, so their `nodePath` is null (not editable).
 */
export function buildViewTree(nodes: ResolvedNode[]): TreeNode[] {
  const walk = (level: ResolvedNode[], prefix: string, indexes: number[] | null, depth: number): TreeNode[] =>
    level.map((node, i): TreeNode => {
      const at = indexes ? [...indexes, i] : null;
      const path = prefix === ROOT ? `${ROOT}${i}` : `${prefix}/${indexes ? '' : 'q'}${i}`;
      if (node.kind === 'page') {
        const row: PageNode = {
          kind: 'page',
          path: null,
          uuid: node.uuid,
          title: node.missing ? 'Missing page' : node.title,
          depth,
          key: `v:${path}`,
          missing: node.missing,
          nodePath: at,
        };
        return row;
      }
      const children = walk(node.children, path, node.kind === 'folder' ? at : null, depth + 1);
      const folder: FolderNode = {
        kind: 'folder',
        path,
        name: node.name,
        depth,
        pinned: false,
        pageCount: countPages(children),
        children,
        nodePath: at,
      };
      if (node.kind === 'query') {
        folder.variant = 'query';
        folder.info = node.error
          ? `error: ${node.error}`
          : node.total > node.children.length
            ? `${node.children.length} of ${node.total}`
            : undefined;
      }
      return folder;
    });
  // Top-level paths are `~0`, `~1`; children extend the parent's path.
  return walk(nodes, ROOT, [], 0);
}

// ── Editing the stored nodes ─────────────────────────────────────────────────

/** The siblings that hold the node at `path`, and its index among them. */
function level(nodes: StoredNode[], path: number[]): { siblings: StoredNode[]; index: number } | null {
  let siblings = nodes;
  for (const i of path.slice(0, -1)) {
    const node = siblings[i];
    if (!node || node.kind !== 'folder') return null;
    siblings = node.children ?? [];
  }
  const index = path[path.length - 1];
  return index !== undefined && index >= 0 && index < siblings.length ? { siblings, index } : null;
}

/** A copy of `nodes` in which the level holding `path` is replaced by `edit`'s result. */
function editLevel(nodes: StoredNode[], path: number[], edit: (siblings: StoredNode[], index: number) => StoredNode[]): StoredNode[] {
  const rebuild = (siblings: StoredNode[], depth: number): StoredNode[] => {
    if (depth === path.length - 1) return edit(siblings, path[depth]);
    return siblings.map((node, i) =>
      i === path[depth] && node.kind === 'folder' ? { ...node, children: rebuild(node.children ?? [], depth + 1) } : node,
    );
  };
  return rebuild(nodes, 0);
}

/** `nodes` without the node at `path`. */
export function removeAt(nodes: StoredNode[], path: number[]): StoredNode[] {
  if (!level(nodes, path)) return nodes;
  return editLevel(nodes, path, (siblings, index) => siblings.filter((_, i) => i !== index));
}

/** Whether the node at `path` can move up (`-1`) or down (`+1`) among its siblings. */
export function canMoveAt(nodes: StoredNode[], path: number[], delta: -1 | 1): boolean {
  const at = level(nodes, path);
  if (!at) return false;
  const target = at.index + delta;
  return target >= 0 && target < at.siblings.length;
}

/** `nodes` with the node at `path` swapped with its neighbour; unchanged at either end. */
export function moveAt(nodes: StoredNode[], path: number[], delta: -1 | 1): StoredNode[] {
  if (!canMoveAt(nodes, path, delta)) return nodes;
  return editLevel(nodes, path, (siblings, index) => {
    const next = [...siblings];
    [next[index], next[index + delta]] = [next[index + delta], next[index]];
    return next;
  });
}

/** `nodes` with a reference to page `uuid` added at the end of the root. */
export function appendPage(nodes: StoredNode[], uuid: string): StoredNode[] {
  return [...nodes, { kind: 'page', uuid }];
}

/** Whether a reference to page `uuid` stands directly in the root of `nodes`. */
export function hasRootPage(nodes: StoredNode[], uuid: string): boolean {
  return nodes.some((n) => n.kind === 'page' && n.uuid === uuid);
}
