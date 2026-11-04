/**
 * The sidebar tree (spec `desktop-page-tree`, design D5, D6, D9).
 *
 * Folders are derived from the pages' `virtualPath` values and never stored.
 * Everything here is pure so it can be tested without a DOM or Tauri.
 */

export interface TreePage {
  uuid: string;
  title: string;
  virtualPath: string | null;
}

export type TreeNode =
  | { kind: 'folder'; path: string; name: string; depth: number; children: TreeNode[] }
  | { kind: 'page'; path: string | null; uuid: string; title: string; depth: number };

/** A row the sidebar renders: a node without its children. */
export type TreeRow =
  | { kind: 'folder'; path: string; name: string; depth: number; expanded: boolean }
  | { kind: 'page'; path: string | null; uuid: string; title: string; depth: number };

const collator = new Intl.Collator(undefined, { sensitivity: 'base', numeric: true });

/** `a/b/c` → `['a', 'a/b', 'a/b/c']`; the root (null or '') has no folders. */
export function ancestorFolders(path: string | null): string[] {
  if (!path) return [];
  const out: string[] = [];
  let prefix = '';
  for (const segment of path.split('/')) {
    prefix = prefix ? `${prefix}/${segment}` : segment;
    out.push(prefix);
  }
  return out;
}

/** Every distinct folder path in the vault, including intermediate ones, sorted. */
export function folderPaths(pages: TreePage[]): string[] {
  const set = new Set<string>();
  for (const page of pages) {
    for (const folder of ancestorFolders(page.virtualPath)) set.add(folder);
  }
  return [...set].sort((a, b) => collator.compare(a, b));
}

/** Folders first, then pages, each group by name. */
export function buildPageTree(pages: TreePage[]): TreeNode[] {
  const root: TreeNode[] = [];
  const folders = new Map<string, Extract<TreeNode, { kind: 'folder' }>>();

  function folderNode(path: string): Extract<TreeNode, { kind: 'folder' }> {
    const existing = folders.get(path);
    if (existing) return existing;
    const segments = path.split('/');
    const name = segments[segments.length - 1];
    const node: Extract<TreeNode, { kind: 'folder' }> = {
      kind: 'folder',
      path,
      name,
      depth: segments.length - 1,
      children: [],
    };
    folders.set(path, node);
    const parentPath = segments.slice(0, -1).join('/');
    (parentPath ? folderNode(parentPath).children : root).push(node);
    return node;
  }

  for (const page of pages) {
    const path = page.virtualPath || null;
    const siblings = path ? folderNode(path).children : root;
    siblings.push({
      kind: 'page',
      path,
      uuid: page.uuid,
      title: page.title,
      depth: path ? path.split('/').length : 0,
    });
  }

  const sortChildren = (nodes: TreeNode[]) => {
    nodes.sort((a, b) => {
      if (a.kind !== b.kind) return a.kind === 'folder' ? -1 : 1;
      const an = a.kind === 'folder' ? a.name : a.title;
      const bn = b.kind === 'folder' ? b.name : b.title;
      return collator.compare(an, bn) || (a.kind === 'page' && b.kind === 'page' ? a.uuid.localeCompare(b.uuid) : 0);
    });
    for (const node of nodes) if (node.kind === 'folder') sortChildren(node.children);
  };
  sortChildren(root);
  return root;
}

/** The rows on screen, in order, skipping everything inside collapsed folders. */
export function flattenVisible(tree: TreeNode[], collapsed: ReadonlySet<string>): TreeRow[] {
  const rows: TreeRow[] = [];
  const walk = (nodes: TreeNode[]) => {
    for (const node of nodes) {
      if (node.kind === 'page') {
        rows.push(node);
        continue;
      }
      const expanded = !collapsed.has(node.path);
      rows.push({ kind: 'folder', path: node.path, name: node.name, depth: node.depth, expanded });
      if (expanded) walk(node.children);
    }
  };
  walk(tree);
  return rows;
}

/** Collapsed paths that still name a folder of `pages`; the rest are stale. */
export function pruneCollapsed(collapsed: Iterable<string>, pages: TreePage[]): string[] {
  const existing = new Set(folderPaths(pages));
  return [...new Set(collapsed)].filter((p) => existing.has(p));
}

/** Expand every folder on the way to `path` (used when a page is opened). */
export function revealPath(collapsed: Iterable<string>, path: string | null): string[] {
  const ancestors = new Set(ancestorFolders(path));
  return [...collapsed].filter((p) => !ancestors.has(p));
}

/** Whether `path` is `prefix` itself or lies under `prefix/`. */
export function isUnder(path: string, prefix: string): boolean {
  return path === prefix || path.startsWith(`${prefix}/`);
}

/** The collapsed set after the folder `from` was renamed to `to`. */
export function carryCollapsed(collapsed: Iterable<string>, from: string, to: string): string[] {
  const out = new Set<string>();
  for (const p of collapsed) {
    if (p === from) out.add(to);
    else if (p.startsWith(`${from}/`)) out.add(`${to}/${p.slice(from.length + 1)}`);
    else out.add(p);
  }
  return [...out];
}

// ── Keyboard (spec `sidebar-navigation`) ─────────────────────────────────────

export type TreeKey = 'next' | 'prev' | 'left' | 'right' | 'enter' | 'space';

export type TreeKeyAction =
  | { kind: 'focus'; index: number }
  | { kind: 'collapse'; path: string }
  | { kind: 'expand'; path: string }
  | { kind: 'toggle'; path: string }
  | { kind: 'open'; uuid: string }
  | null;

/** The index of the folder row that contains the row at `index`, if any. */
export function parentRowIndex(rows: TreeRow[], index: number): number {
  const row = rows[index];
  if (!row || row.depth === 0) return -1;
  for (let i = index - 1; i >= 0; i--) {
    const candidate = rows[i];
    if (candidate.kind === 'folder' && candidate.depth === row.depth - 1) return i;
  }
  return -1;
}

/**
 * What a key does on the focused row. `index` is -1 when no row has focus, in
 * which case `next` and `prev` land on the first and last row.
 */
export function treeKeyAction(rows: TreeRow[], index: number, key: TreeKey): TreeKeyAction {
  if (rows.length === 0) return null;
  const row = rows[index];

  switch (key) {
    case 'next':
      return { kind: 'focus', index: index < 0 ? 0 : (index + 1) % rows.length };
    case 'prev':
      return { kind: 'focus', index: index < 0 ? rows.length - 1 : (index - 1 + rows.length) % rows.length };
    case 'left': {
      if (!row) return null;
      if (row.kind === 'folder' && row.expanded) return { kind: 'collapse', path: row.path };
      const parent = parentRowIndex(rows, index);
      return parent >= 0 ? { kind: 'focus', index: parent } : null;
    }
    case 'right': {
      if (!row || row.kind !== 'folder') return null;
      if (!row.expanded) return { kind: 'expand', path: row.path };
      const child = rows[index + 1];
      return child && child.depth > row.depth ? { kind: 'focus', index: index + 1 } : null;
    }
    case 'enter':
      if (!row) return null;
      return row.kind === 'folder' ? { kind: 'toggle', path: row.path } : { kind: 'open', uuid: row.uuid };
    case 'space':
      return row?.kind === 'folder' ? { kind: 'toggle', path: row.path } : null;
  }
}

/** Page UUIDs in sidebar order, visible rows only (for next/previous page). */
export function visiblePageUuids(rows: TreeRow[]): string[] {
  return rows.flatMap((r) => (r.kind === 'page' ? [r.uuid] : []));
}
