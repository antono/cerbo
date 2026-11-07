// Keystroke-path budget for the page editor (spec `editor-input-performance`).
//
// Carta's `MarkdownEditor` mounts its `Renderer` even while the preview tab is
// hidden, and the renderer re-parses the whole page on every value change: a
// synchronous `renderSSR` in its template plus a debounced async `render`.
// Carta's `Input` diffs the whole document against the last highlighted text
// on every key (`speculativeHighlightUpdate` → `diffChars`) and rewrites the
// shiki overlay. Both are O(document) per keystroke. This module keeps the
// knobs and the render gate in one place so a Carta upgrade has one spot to
// re-check.

/**
 * Pages at least this long (in UTF-16 code units, about bytes for mostly
 * ASCII markdown) are edited in a plain textarea without the shiki overlay,
 * and so without syntax colours.
 *
 * Measured in JavaScriptCore (`bun scripts/bench-overlay-diff.ts`, same engine
 * as WebKitGTK, numbers in design.md Context): Carta's per-key `diffChars`
 * alone reaches the 16 ms frame budget at p95 from about 75–100 KB and costs
 * ~2–5 ms at 50 KB, before the overlay's DOM rewrite. 50 KB keeps a margin
 * for that DOM work until a WebKit Inspector trace narrows it.
 */
export const LARGE_DOCUMENT_CHARS = 50 * 1024;

/**
 * Debounce for the full shiki re-highlight of the input overlay (Carta's
 * default is 250 ms). Longer, so that the overlay is re-highlighted after a
 * pause rather than between two keys of a slow typist.
 */
export const OVERLAY_HIGHLIGHT_DELAY_MS = 500;

/**
 * A page edited in the plain textarea goes back to the overlay only once it
 * shrinks below this, so a page hovering around the threshold does not swap
 * editors back and forth.
 */
export const LARGE_DOCUMENT_EXIT_CHARS = 40 * 1024;

/**
 * Idle time after the last change before the input mode is re-checked for a
 * page that grew (or shrank) past the threshold while open: a paste, typing or
 * an external reload. Never on the keystroke path.
 */
export const INPUT_MODE_RECHECK_MS = 1000;

/** Whether a page of `length` characters is edited without the overlay. */
export function usePlainInput(length: number): boolean {
  return length >= LARGE_DOCUMENT_CHARS;
}

/** Input mode for an open page after it changed to `length` characters. */
export function nextPlainInput(current: boolean, length: number): boolean {
  return current ? length >= LARGE_DOCUMENT_EXIT_CHARS : usePlainInput(length);
}

/** Caret, selection and scroll of the write-mode input, kept across an editor swap. */
export interface InputView {
  selectionStart: number;
  selectionEnd: number;
  scrollTop: number;
  focused: boolean;
}

/**
 * The element that scrolls the write-mode input: the plain textarea scrolls
 * itself, Carta's overlay input scrolls its `.carta-input` container.
 */
export function inputScroller(textarea: HTMLTextAreaElement): HTMLElement {
  if (textarea.classList.contains('carta-plain-input')) return textarea;
  return textarea.closest<HTMLElement>('.carta-input') ?? textarea;
}

export function captureInputView(textarea: HTMLTextAreaElement): InputView {
  return {
    selectionStart: textarea.selectionStart,
    selectionEnd: textarea.selectionEnd,
    scrollTop: inputScroller(textarea).scrollTop,
    focused: textarea.ownerDocument.activeElement === textarea,
  };
}

export function restoreInputView(textarea: HTMLTextAreaElement, view: InputView): void {
  if (view.focused) textarea.focus({ preventScroll: true });
  textarea.setSelectionRange(view.selectionStart, view.selectionEnd);
  inputScroller(textarea).scrollTop = view.scrollTop;
}

/** The part of Carta's `InputEnhancer` that places popups at the caret. */
export interface CaretTarget {
  textarea: HTMLTextAreaElement;
  getCursorXY(selectionPoint?: number): { x: number; y: number; left: number; top: number; right: number; bottom: number };
}

/**
 * Carta measures the caret from the top of the text and expects the textarea
 * to be as tall as its content (its container scrolls). The plain textarea
 * scrolls itself, so popups bound to the caret (emoji picker, `bindToCaret`)
 * would land below the visible box on a scrolled page. Subtracts the
 * textarea's own scroll offset from every caret position.
 */
export function offsetCaretByTextareaScroll(input: CaretTarget): void {
  const getCursorXY = input.getCursorXY;
  input.getCursorXY = function (selectionPoint?: number) {
    const xy = getCursorXY.call(this, selectionPoint ?? input.textarea.selectionStart);
    const dx = input.textarea.scrollLeft;
    const dy = input.textarea.scrollTop;
    return { ...xy, x: xy.x - dx, left: xy.left - dx, y: xy.y - dy, top: xy.top - dy };
  };
}

/** The part of a Carta instance the render gate replaces. */
export interface RenderTarget {
  render(markdown: string): Promise<string>;
  renderSSR(markdown: string): string;
}

/**
 * Makes `carta.render` and `carta.renderSSR` return an empty string while
 * `isPreviewVisible()` is false, so the hidden preview of the write tab does
 * no markdown work during typing. The preview tab remounts the renderer
 * (`{#key}` in `PageEditor`), which renders the current text in full once the
 * gate is open. Returns a function that restores the original methods.
 */
export function gatePreviewRendering(carta: RenderTarget, isPreviewVisible: () => boolean): () => void {
  const render = carta.render;
  const renderSSR = carta.renderSSR;

  carta.render = function (markdown: string) {
    return isPreviewVisible() ? render.call(this, markdown) : Promise.resolve('');
  };
  carta.renderSSR = function (markdown: string) {
    return isPreviewVisible() ? renderSSR.call(this, markdown) : '';
  };

  return () => {
    carta.render = render;
    carta.renderSSR = renderSSR;
  };
}
