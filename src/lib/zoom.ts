/**
 * Interface zoom (the whole webview scales, as with a browser's zoom).
 *
 * Pure step and clamp logic, free of Svelte and Tauri so vitest runs it. The
 * app applies the factor with the Tauri webview zoom API (`action-host.ts`)
 * and keeps it in `ui.toml` across restarts.
 */

export const ZOOM_MIN = 0.5;
export const ZOOM_MAX = 2;
export const ZOOM_DEFAULT = 1;

/** The levels Zoom in and Zoom out step through, like a browser's. */
export const ZOOM_LEVELS: readonly number[] = [0.5, 0.67, 0.75, 0.8, 0.9, 1, 1.1, 1.25, 1.5, 1.75, 2];

/** `zoom` kept within `ZOOM_MIN`..`ZOOM_MAX`; anything not a finite number is 100%. */
export function clampZoom(zoom: number): number {
  if (!Number.isFinite(zoom) || zoom <= 0) return ZOOM_DEFAULT;
  return Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, zoom));
}

/** A stored zoom (from `ui.toml`, maybe absent or edited by hand) as a usable factor. */
export function parseZoom(value: unknown): number {
  return typeof value === 'number' ? clampZoom(value) : ZOOM_DEFAULT;
}

// Levels are rounded to two places, so compare with some slack.
const EPS = 1e-6;

/** The next level above `zoom`, or `ZOOM_MAX` when there is none. */
export function zoomIn(zoom: number): number {
  const z = clampZoom(zoom);
  return ZOOM_LEVELS.find((level) => level > z + EPS) ?? ZOOM_MAX;
}

/** The next level below `zoom`, or `ZOOM_MIN` when there is none. */
export function zoomOut(zoom: number): number {
  const z = clampZoom(zoom);
  return [...ZOOM_LEVELS].reverse().find((level) => level < z - EPS) ?? ZOOM_MIN;
}

/** `1.25` -> `125%`, for menu labels. */
export function zoomPercent(zoom: number): string {
  return `${Math.round(clampZoom(zoom) * 100)}%`;
}
