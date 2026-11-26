/**
 * Width classes and panel overlays (change `unify-window-toolbar`, spec
 * `desktop-responsive-layout`, design D4). Pure functions, free of Svelte and
 * Tauri, so vitest covers every transition.
 *
 * `showLeft` / `showRight` are the docked state of the wide layout; they are
 * persisted. `overlay` is the one panel floating over the editor (medium right
 * panel, narrow drawer and sheet); it is never persisted.
 */

export type WidthClass = 'wide' | 'medium' | 'narrow';
export type Panel = 'tree' | 'right';
export type Overlay = Panel | null;

/** Wide from this width up. */
export const WIDE_MIN = 1000;
/** Medium from this width up; narrow below. */
export const MEDIUM_MIN = 700;
/** The editor never gets narrower than this (or the window, when smaller). */
export const EDITOR_MIN = 360;

export function widthClassFor(width: number): WidthClass {
  if (width >= WIDE_MIN) return 'wide';
  if (width >= MEDIUM_MIN) return 'medium';
  return 'narrow';
}

export interface PanelState {
  widthClass: WidthClass;
  showLeft: boolean;
  showRight: boolean;
  overlay: Overlay;
}

/**
 * Whether `panel` is docked beside the editor in the current class. The tree
 * docks in wide and medium; the right panel only in wide.
 */
export function isDocked(panel: Panel, widthClass: WidthClass): boolean {
  if (widthClass === 'wide') return true;
  return widthClass === 'medium' && panel === 'tree';
}

/** Whether `panel` is on screen, docked or as the open overlay. */
export function panelShown(panel: Panel, s: PanelState): boolean {
  if (isDocked(panel, s.widthClass)) return panel === 'tree' ? s.showLeft : s.showRight;
  return s.overlay === panel;
}

/**
 * The state after the user uses `panel`'s toggle. A docked panel flips its
 * stored flag (`persist` true). An overlay panel opens, closing any other
 * overlay, or closes when it is already open; the stored flags stay as they
 * were.
 */
export function togglePanel(panel: Panel, s: PanelState): { state: PanelState; persist: boolean } {
  if (isDocked(panel, s.widthClass)) {
    const state =
      panel === 'tree'
        ? { ...s, showLeft: !s.showLeft, overlay: null }
        : { ...s, showRight: !s.showRight, overlay: null };
    return { state, persist: true };
  }
  return { state: { ...s, overlay: s.overlay === panel ? null : panel }, persist: false };
}

/** The state after `panel` is made visible (Reveal in tree, folder segment). */
export function showPanel(panel: Panel, s: PanelState): { state: PanelState; persist: boolean } {
  return panelShown(panel, s) ? { state: s, persist: false } : togglePanel(panel, s);
}

/** The state after the window crosses into `widthClass`: overlays close. */
export function changeWidthClass(widthClass: WidthClass, s: PanelState): PanelState {
  if (widthClass === s.widthClass) return s;
  return { ...s, widthClass, overlay: null };
}

/**
 * The widest a docked side panel may get so the editor keeps `EDITOR_MIN`
 * (spec "Editor keeps its width"). `other` is the width of the other docked
 * panel, 0 when hidden. Never below `min`, the panel's own minimum.
 */
export function maxPanelWidth(windowWidth: number, other: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, windowWidth - other - EDITOR_MIN));
}

/** Minimum and maximum widths of the docked tree and right panel. */
export const TREE_MIN = 200;
export const RIGHT_MIN = 250;
export const PANEL_MAX = 600;

/**
 * The widest the docked right panel (agent chat, links...) may be dragged:
 * half the window, never less than `PANEL_MAX`. `maxPanelWidth` still keeps
 * the editor its `EDITOR_MIN`.
 */
export function rightPanelMax(windowWidth: number): number {
  return Math.max(PANEL_MAX, Math.floor(windowWidth / 2));
}

/**
 * The widths the docked panels get in a window `windowWidth` wide, so the
 * editor keeps `EDITOR_MIN` (spec "Editor keeps its width": side panels give
 * way to the editor). `tree` and `right` are the stored widths, 0 when that
 * panel is not docked. The right panel gives way first, down to its minimum,
 * then the tree. The stored widths stay as they are, so a wider window gets
 * them back. A `windowWidth` of 0 (not measured yet) leaves them unchanged.
 */
export function dockedWidths(windowWidth: number, tree: number, right: number): { tree: number; right: number } {
  if (windowWidth <= 0) return { tree, right };
  const room = windowWidth - EDITOR_MIN;
  let r = right;
  let t = tree;
  if (t + r > room && r > 0) r = Math.max(Math.min(r, RIGHT_MIN), room - t);
  if (t + r > room && t > 0) t = Math.max(Math.min(t, TREE_MIN), room - r);
  return { tree: t, right: r };
}
