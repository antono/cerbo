/**
 * The sidebar tree (spec `desktop-page-tree`, design D5, D6, D9;
 * `add-sidebar-ordering` D2, D6).
 *
 * Folders come from the pages' `virtualPath` values plus the folders pinned in
 * the vault layout (`.cerbo/tree.toml`), whose `order` lists also set the
 * order of each folder's children. Everything here is pure so it can be
 * tested without a DOM or Tauri.
 */

export interface TreePage {
  uuid: string;
  title: string;
  virtualPath: string | null;
}

/** The vault layout file, as the `tree_layout` command returns it. */
export interface TreeLayout {
  pinned: string[];
  order: Record<string, string[]>;
}

export const EMPTY_LAYOUT: TreeLayout = { pinned: [], order: {} };

export type FolderNode = {
  kind: 'folder';
  path: string;
  name: string;
  depth: number;
  pinned: boolean;
  /** Pages at or under this folder. */
  pageCount: number;
  children: TreeNode[];
  /** Views only (`view-tree.ts`): a query node shows the pages a SPARQL query finds. */
  variant?: 'query';
  /** Views only: a short note after the name, such as a query's error or row cap. */
  info?: string;
  /** Views only: where the stored node sits (indexes from the root), null when not editable. */
  nodePath?: number[] | null;
};
export type PageNode = {
  kind: 'page';
  path: string | null;
  uuid: string;
  title: string;
  depth: number;
  /** Views only: unique row key, since a page may appear many times in one view. */
  key?: string;
  /** Views only: the page no longer exists. */
  missing?: boolean;
  /** Views only: where the stored node sits (indexes from the root), null when not editable. */
  nodePath?: number[] | null;
};
export type TreeNode = FolderNode | PageNode;

/** A row the sidebar renders: a node without its children. */
export type TreeRow =
  | {
      kind: 'folder';
      path: string;
      name: string;
      depth: number;
      expanded: boolean;
      pinned: boolean;
      pageCount: number;
      hasChildren: boolean;
      variant?: 'query';
      info?: string;
      nodePath?: number[] | null;
    }
  | PageNode;

const isDigit = (c: number) => c >= 48 && c <= 57;
const codePoints = (s: string) => Array.from(s, (c) => c.codePointAt(0) as number);

/**
 * The default order of two names or titles. Letter case is ignored and a run
 * of ASCII digits counts as a number, so "Page 2" comes before "Page 10";
 * everything else goes by Unicode code point. Names equal that way fall back
 * to plain code point order. It depends on no locale, and `compare_names` in
 * `core/src/tree.rs` is the same rule: change both together.
 */
export function compareNames(a: string, b: string): number {
  const la = codePoints(a.toLowerCase());
  const lb = codePoints(b.toLowerCase());
  let i = 0;
  let j = 0;
  while (i < la.length && j < lb.length) {
    if (isDigit(la[i]) && isDigit(lb[j])) {
      let endA = i;
      let endB = j;
      while (endA < la.length && isDigit(la[endA])) endA++;
      while (endB < lb.length && isDigit(lb[endB])) endB++;
      // Without leading zeros the longer number is the larger one.
      while (i < endA - 1 && la[i] === 48) i++;
      while (j < endB - 1 && lb[j] === 48) j++;
      if (endA - i !== endB - j) return endA - i - (endB - j);
      for (; i < endA; i++, j++) if (la[i] !== lb[j]) return la[i] - lb[j];
    } else if (la[i] !== lb[j]) {
      return la[i] - lb[j];
    } else {
      i++;
      j++;
    }
  }
  return la.length - i - (lb.length - j) || compareCodePoints(a, b);
}

/** Plain code point order, as Rust compares strings (not UTF-16 unit order). */
function compareCodePoints(a: string, b: string): number {
  const ca = codePoints(a);
  const cb = codePoints(b);
  for (let i = 0; i < ca.length && i < cb.length; i++) if (ca[i] !== cb[i]) return ca[i] - cb[i];
  return ca.length - cb.length;
}

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

/** Every distinct folder path in the vault, including intermediate ones and pinned ones, sorted. */
export function folderPaths(pages: TreePage[], layout: TreeLayout = EMPTY_LAYOUT): string[] {
  const set = new Set<string>();
  for (const page of pages) {
    for (const folder of ancestorFolders(page.virtualPath)) set.add(folder);
  }
  for (const pinned of layout.pinned) {
    for (const folder of ancestorFolders(pinned)) set.add(folder);
  }
  return [...set].sort(compareNames);
}

/** The parent folder of a folder path; `''` for a top-level folder. */
export function parentPath(path: string): string {
  const i = path.lastIndexOf('/');
  return i < 0 ? '' : path.slice(0, i);
}

/** The key a node or row has in its parent's order list (D1). */
export function entryKey(node: { kind: 'folder'; name: string } | { kind: 'page'; uuid: string }): string {
  return node.kind === 'folder' ? `${node.name}/` : node.uuid;
}

/** The folder a node or row lives in; `''` is the root. */
export function parentOf(node: TreeNode | TreeRow): string {
  return node.kind === 'folder' ? parentPath(node.path) : (node.path ?? '');
}

/**
 * The tree: each folder's children named in the layout's order list come
 * first, in list order; the rest follow, folders before pages, each by name.
 */
export function buildPageTree(pages: TreePage[], layout: TreeLayout = EMPTY_LAYOUT): TreeNode[] {
  const root: TreeNode[] = [];
  const folders = new Map<string, FolderNode>();
  const pinned = new Set(layout.pinned);

  function folderNode(path: string): FolderNode {
    const existing = folders.get(path);
    if (existing) return existing;
    const segments = path.split('/');
    const name = segments[segments.length - 1];
    const node: FolderNode = {
      kind: 'folder',
      path,
      name,
      depth: segments.length - 1,
      pinned: pinned.has(path),
      pageCount: 0,
      children: [],
    };
    folders.set(path, node);
    const parentPath = segments.slice(0, -1).join('/');
    (parentPath ? folderNode(parentPath).children : root).push(node);
    return node;
  }

  for (const path of layout.pinned) folderNode(path);

  for (const page of pages) {
    const path = page.virtualPath || null;
    for (const folder of ancestorFolders(path)) folderNode(folder).pageCount++;
    const siblings = path ? folderNode(path).children : root;
    siblings.push({
      kind: 'page',
      path,
      uuid: page.uuid,
      title: page.title,
      depth: path ? path.split('/').length : 0,
    });
  }

  const sortChildren = (nodes: TreeNode[], parent: string) => {
    nodes.sort((a, b) => {
      if (a.kind !== b.kind) return a.kind === 'folder' ? -1 : 1;
      const an = a.kind === 'folder' ? a.name : a.title;
      const bn = b.kind === 'folder' ? b.name : b.title;
      return compareNames(an, bn) || (a.kind === 'page' && b.kind === 'page' ? compareCodePoints(a.uuid, b.uuid) : 0);
    });
    const list = layout.order[parent];
    if (list && list.length > 0) {
      const rest = [...nodes];
      const ordered: TreeNode[] = [];
      for (const key of list) {
        const i = rest.findIndex((n) => entryKey(n) === key);
        if (i >= 0) ordered.push(...rest.splice(i, 1));
      }
      nodes.splice(0, nodes.length, ...ordered, ...rest);
    }
    for (const node of nodes) if (node.kind === 'folder') sortChildren(node.children, node.path);
  };
  sortChildren(root, '');
  return root;
}

/** The children of the folder at `path` (`''` is the root), or null if there is no such folder. */
export function childrenOf(tree: TreeNode[], path: string): TreeNode[] | null {
  if (path === '') return tree;
  let level = tree;
  let prefix = '';
  for (const segment of path.split('/')) {
    prefix = prefix ? `${prefix}/${segment}` : segment;
    const next = level.find((n): n is FolderNode => n.kind === 'folder' && n.path === prefix);
    if (!next) return null;
    level = next.children;
  }
  return level;
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
      rows.push({
        kind: 'folder',
        path: node.path,
        name: node.name,
        depth: node.depth,
        expanded,
        pinned: node.pinned,
        pageCount: node.pageCount,
        hasChildren: node.children.length > 0,
        variant: node.variant,
        info: node.info,
        nodePath: node.nodePath,
      });
      if (expanded) walk(node.children);
    }
  };
  walk(tree);
  return rows;
}

/** Collapsed paths that still name a folder of `pages` or the layout; the rest are stale. */
export function pruneCollapsed(collapsed: Iterable<string>, pages: TreePage[], layout: TreeLayout = EMPTY_LAYOUT): string[] {
  const existing = new Set(folderPaths(pages, layout));
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


// ── Placing rows (add-sidebar-ordering, design D6) ──────────────────────────

/** What is being placed: a page or a folder. */
export type TreeItem = { kind: 'page'; uuid: string; path: string | null } | { kind: 'folder'; path: string };

export function itemOf(row: TreeRow): TreeItem {
  return row.kind === 'page' ? { kind: 'page', uuid: row.uuid, path: row.path } : { kind: 'folder', path: row.path };
}

/** The store calls a placement needs: an optional move, then the parent's new order. */
export interface Placement {
  move: { kind: 'page'; uuid: string; to: string | null } | { kind: 'folder'; from: string; to: string } | null;
  /** The folder whose order list is written (`''` is the root). */
  parent: string;
  /** The parent's children after the placement, as entry keys. */
  order: string[];
}

function itemKey(item: TreeItem): string {
  if (item.kind === 'page') return item.uuid;
  const i = item.path.lastIndexOf('/');
  return `${item.path.slice(i + 1)}/`;
}

function itemParent(item: TreeItem): string {
  return item.kind === 'page' ? (item.path ?? '') : parentPath(item.path);
}

/**
 * Put `item` into the folder `parent` before the child now shown at `index`
 * (`index` = number of children puts it last). Returns null when nothing would
 * change, when `parent` does not exist, or when a folder would go inside
 * itself.
 */
export function placement(tree: TreeNode[], item: TreeItem, parent: string, index: number): Placement | null {
  if (item.kind === 'folder' && (parent === item.path || isUnder(parent, item.path))) return null;
  const children = childrenOf(tree, parent);
  if (!children) return null;
  const keys = children.map(entryKey);
  const key = itemKey(item);
  const sameParent = itemParent(item) === parent;
  let at = Math.max(0, Math.min(index, keys.length));
  const from = keys.indexOf(key);
  if (from >= 0) {
    // Same folder, or a folder of the same name that a moved folder merges into.
    keys.splice(from, 1);
    if (from < at) at--;
  }
  const order = [...keys.slice(0, at), key, ...keys.slice(at)];
  if (sameParent && from === at) return null;

  let move: Placement['move'] = null;
  if (!sameParent) {
    if (item.kind === 'page') {
      move = { kind: 'page', uuid: item.uuid, to: parent || null };
    } else {
      const name = key.slice(0, -1);
      move = { kind: 'folder', from: item.path, to: parent ? `${parent}/${name}` : name };
    }
  }
  return { move, parent, order };
}

/** Move a row one place up (`-1`) or down (`+1`) among its siblings; null at either end. */
export function moveBy(tree: TreeNode[], row: TreeRow, delta: -1 | 1): Placement | null {
  const parent = parentOf(row);
  const siblings = childrenOf(tree, parent);
  if (!siblings) return null;
  const i = siblings.findIndex((n) => entryKey(n) === entryKey(row));
  const j = i + delta;
  if (i < 0 || j < 0 || j >= siblings.length) return null;
  return placement(tree, itemOf(row), parent, delta > 0 ? i + 2 : j);
}

/** Whether a row can move up (`-1`) or down (`+1`) among its siblings. */
export function canMoveBy(tree: TreeNode[], row: TreeRow, delta: -1 | 1): boolean {
  return moveBy(tree, row, delta) !== null;
}
