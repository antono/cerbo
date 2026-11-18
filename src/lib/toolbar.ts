/**
 * Pure helpers behind `WindowToolbar.svelte` (change `unify-window-toolbar`,
 * spec `desktop-window-toolbar`). Free of Svelte and Tauri so vitest runs them.
 */

import { hotkeyText, type Hotkey } from './actions';
import { isMac } from './hotkeys';

/** One folder segment of the open page's path; `path` is the folder it reveals. */
export interface PathSegment {
  name: string;
  path: string;
}

/** Folder segments of a page's virtual path: `a/b` -> `a` (a), `b` (a/b). */
export function pathSegments(virtualPath: string | null | undefined): PathSegment[] {
  if (!virtualPath) return [];
  const out: PathSegment[] = [];
  let prefix = '';
  for (const name of virtualPath.split('/')) {
    if (!name) continue;
    prefix = prefix ? `${prefix}/${name}` : name;
    out.push({ name, path: prefix });
  }
  return out;
}

/** The folder holding `path`, or null at the root: `a/b` -> `a`, `a` -> null. */
export function parentFolder(path: string): string | null {
  const i = path.lastIndexOf('/');
  return i < 0 ? null : path.slice(0, i);
}

/** Tooltip text: the action name, then its hotkey in parentheses when it has one. */
export function controlTitle(label: string, hotkey?: Hotkey, mac = isMac): string {
  return hotkey ? `${label} (${hotkeyText(hotkey, mac)})` : label;
}

/**
 * Whether a click on a menu button should open its menu. The menu closes on
 * the button's own pointerdown (outside-click handling), so a click that comes
 * right after its menu closed means "close", not "open again".
 */
export function shouldOpenMenu(
  kind: string,
  lastClosed: { kind: string; at: number } | null,
  now: number,
  window = 300,
): boolean {
  return !(lastClosed && lastClosed.kind === kind && now - lastClosed.at < window);
}

/**
 * The revealed folder after tree focus moves to the row `rowKey` (`f:<path>`
 * for a folder, null for no row). It stays only while its own row has focus.
 */
export function revealedAfterFocus(revealed: string | null, rowKey: string | null): string | null {
  if (revealed === null || rowKey === null) return revealed;
  return rowKey === `f:${revealed}` ? revealed : null;
}

/**
 * Entries of the ⋯ menu for a width class. The narrow toolbar drops forward
 * and the right panel toggle, so ⋯ offers them first (spec
 * `desktop-responsive-layout`, "Narrow toolbar"); new page lives in the
 * drawer and the mode toggle floats over the editor.
 */
export function overflowMenuIds(base: readonly string[], widthClass: 'wide' | 'medium' | 'narrow'): string[] {
  return widthClass === 'narrow' ? ['nav.forward', 'panel.right', ...base] : [...base];
}
