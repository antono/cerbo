/**
 * The seam between the Outline sub-panel and the editor (change
 * `add-right-panel-tabs`, design D5). The page editor owns the textarea and the
 * rendered preview, so it registers what it can do here; the panel only asks.
 * Mirrors `onEditorLeave` in the store: one registration, no component imports
 * the other.
 */

export interface OutlineBridge {
  /**
   * Scroll the editor (or the preview, in preview mode) so heading `index` of
   * the page's headings is at the top, and in edit mode put the cursor there.
   */
  jumpTo(index: number): void;
}

let bridge: OutlineBridge | null = null;

/** Register the open editor's bridge; the returned function removes it. */
export function setOutlineBridge(next: OutlineBridge): () => void {
  bridge = next;
  return () => {
    if (bridge === next) bridge = null;
  };
}

/** Jump to heading `index`; does nothing while no editor is open. */
export function jumpToHeading(index: number): void {
  bridge?.jumpTo(index);
}
