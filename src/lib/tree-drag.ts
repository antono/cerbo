/**
 * Pointer-event drag in the sidebar tree (`add-virtual-path-tree` D8,
 * `add-sidebar-ordering` D7). Pure geometry and decisions; the component owns
 * the listeners.
 */
import {
  childrenOf,
  entryKey,
  isUnder,
  parentOf,
  placement,
  type Placement,
  type TreeItem,
  type TreeNode,
  type TreeRow,
} from './page-tree';

/** Pixels the pointer must travel with the button held before a drag starts. */
export const DRAG_THRESHOLD = 4;

export type DropZone = 'before' | 'after' | 'inside-last' | 'inside-first';

/** Whether the pointer moved far enough from where the button went down. */
export function passedThreshold(dx: number, dy: number): boolean {
  return Math.hypot(dx, dy) > DRAG_THRESHOLD;
}

/**
 * The zone of `row` under the pointer at height `y`, for a row spanning
 * `top`..`top + height` (design D7).
 */
export function dropZone(row: TreeRow, top: number, height: number, y: number): DropZone {
  const rel = height > 0 ? (y - top) / height : 0.5;
  if (row.kind === 'page') return rel < 0.5 ? 'before' : 'after';
  if (rel < 0.25) return 'before';
  if (rel > 0.75) return row.expanded && row.hasChildren ? 'inside-first' : 'after';
  return 'inside-last';
}

/** Where a drop on `target` in `zone` puts `item`, or null when it is refused or changes nothing. */
export function dropPlacement(tree: TreeNode[], item: TreeItem, target: TreeRow, zone: DropZone): Placement | null {
  if (item.kind === 'folder' && target.kind === 'folder' && isUnder(target.path, item.path)) return null;
  if (zone === 'inside-last' || zone === 'inside-first') {
    if (target.kind !== 'folder') return null;
    const children = childrenOf(tree, target.path) ?? [];
    return placement(tree, item, target.path, zone === 'inside-first' ? 0 : children.length);
  }
  const parent = parentOf(target);
  const siblings = childrenOf(tree, parent);
  if (!siblings) return null;
  const at = siblings.findIndex((n) => entryKey(n) === entryKey(target));
  if (at < 0) return null;
  return placement(tree, item, parent, zone === 'before' ? at : at + 1);
}

/** A drop on the zone below the tree: last at the root. */
export function rootPlacement(tree: TreeNode[], item: TreeItem): Placement | null {
  return placement(tree, item, '', tree.length);
}
