/**
 * Applies the interface zoom (`zoom.ts`) to the app's webview. The webview
 * zoom scales everything, layout included, like a browser's zoom: CSS pixels
 * grow, so the window is narrower in CSS pixels and the responsive layout
 * reacts to it on its own.
 */

import { clampZoom } from './zoom';

export async function applyZoom(zoom: number): Promise<void> {
  try {
    const { getCurrentWebview } = await import('@tauri-apps/api/webview');
    await getCurrentWebview().setZoom(clampZoom(zoom));
  } catch (e) {
    console.error('Failed to set the interface zoom:', e);
  }
}
