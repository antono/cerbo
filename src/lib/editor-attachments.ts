/**
 * Drop and paste into the page editor (spec `editor-attachment-drop`, design
 * Decision 4). One pipeline for every source:
 *
 * - OS file drops arrive as Tauri drag-drop events with absolute paths, which
 *   Rust copies directly (`attachment_add`); no bytes cross IPC.
 * - Fragment drops (image, HTML, text from another window) arrive as HTML5
 *   `drop` events; their bytes are uploaded raw (`attachment_upload`). An
 *   image dragged out of a web page with no file attached is downloaded by
 *   URL (`attachment_fetch`), as is a web URL that a native drop reports as a
 *   path (wry on Linux turns every `text/uri-list` entry into one).
 * - Pastes use the same two commands: copied files by path, image data by bytes.
 *   A pasted bare path list asks Rust for the clipboard's files
 *   (`clipboard_file_paths`) in case the webview hid the file types.
 *
 * Each stored Attachment gets a `![name](cerbo://objects/<uuid>)` (images) or
 * `[name](cerbo://objects/<uuid>)` reference at the drop point or caret. A
 * failed copy is reported and inserts nothing. The pure parts live in
 * `attachment-drop.ts`.
 */

import { getCurrentWebview } from '@tauri-apps/api/webview';
import {
  attachmentReference,
  baseName,
  classifyDrop,
  classifyPaste,
  decodeDataUrl,
  dropPositionToClient,
  fragmentName,
  isWebUrl,
  looksLikeCopiedPaths,
  nativeDropSource,
  rebaseRange,
  type DropContent,
} from './attachment-drop';

/** Where the drop indicator is drawn, relative to the editor container. */
export interface DropIndicator {
  left: number;
  top: number;
  height: number;
}

export interface EditorAttachmentOptions {
  /** The editor's root element; listeners are attached here. */
  container: HTMLElement;
  /** Carta's textarea, when mounted. */
  getTextarea: () => HTMLTextAreaElement | null | undefined;
  /** Identifies the open page; a reference is only inserted into the page it was dropped on. */
  getPageKey: () => string | null;
  /** True while the editor is in write mode with a page open. */
  isActive: () => boolean;
  /** Copy a file into the vault by path; resolves to the new object's UUID. */
  addPath: (path: string) => Promise<string>;
  /** Store bytes as a new Attachment; resolves to the new object's UUID. */
  addBytes: (name: string, bytes: Uint8Array) => Promise<string>;
  /**
   * Download a web URL as a new Attachment, named after the URL or else
   * `fallbackName`; resolves to its UUID and file name.
   */
  addUrl: (url: string, fallbackName: string) => Promise<StoredAttachment>;
  /** Local paths of files copied in a file manager, read natively (empty if none). */
  readClipboardPaths: () => Promise<string[]>;
  /** Called after references were inserted. */
  onInserted: () => void;
  /** Called with a message naming each file that could not be stored. */
  onError: (message: string) => void;
  /** Show (or with null, hide) the drop indicator. */
  onIndicator: (indicator: DropIndicator | null) => void;
}

/** A stored Attachment: its object UUID and the file name it was stored under. */
export interface StoredAttachment {
  uuid: string;
  name: string;
}

/** One thing to store: its name (for messages) and how to store it. */
export interface PendingAttachment {
  name: string;
  store: () => Promise<StoredAttachment>;
}

/** Where references go: a range of the textarea's value as it was when taken. */
export interface InsertionPoint {
  start: number;
  end: number;
  /** The textarea's value when the range was taken. */
  value: string;
}

// ── Point → textarea offset ──────────────────────────────────────────────────

const MIRROR_STYLES = [
  'fontFamily', 'fontSize', 'fontWeight', 'fontStyle', 'fontVariant', 'fontFeatureSettings',
  'textTransform', 'textIndent', 'wordSpacing', 'letterSpacing', 'lineHeight', 'tabSize',
  'paddingLeft', 'paddingTop', 'paddingRight', 'paddingBottom',
  'borderLeftWidth', 'borderTopWidth', 'borderRightWidth', 'borderBottomWidth',
  'borderStyle', 'boxSizing', 'wordBreak', 'overflowWrap',
] as const;

/**
 * Text offset and caret rectangle under a client point in `textarea`, found
 * with an invisible mirror laid exactly over it and the browser's own
 * caret-from-point hit test. Null when the point is outside or the browser
 * cannot tell; callers fall back to the caret.
 */
export function textareaHitTest(
  textarea: HTMLTextAreaElement,
  x: number,
  y: number,
): { offset: number; rect: { left: number; top: number; height: number } } | null {
  const box = textarea.getBoundingClientRect();
  if (x < box.left || x > box.right || y < box.top || y > box.bottom) return null;

  const style = window.getComputedStyle(textarea);
  const mirror = document.createElement('div');
  for (const prop of MIRROR_STYLES) {
    mirror.style[prop] = style[prop];
  }
  Object.assign(mirror.style, {
    position: 'fixed',
    left: `${box.left}px`,
    top: `${box.top}px`,
    width: `${box.width}px`,
    height: `${box.height}px`,
    margin: '0',
    whiteSpace: 'pre-wrap',
    overflow: 'hidden',
    opacity: '0',
    zIndex: '2147483647',
    pointerEvents: 'auto',
  });
  // The trailing space gives a final empty line a box to hit.
  const text = document.createTextNode(`${textarea.value} `);
  mirror.appendChild(text);
  document.body.appendChild(mirror);
  mirror.scrollTop = textarea.scrollTop;

  try {
    const doc = document as Document & {
      caretPositionFromPoint?: (x: number, y: number) => { offsetNode: Node; offset: number } | null;
    };
    let node: Node | null = null;
    let offset = 0;
    if (typeof doc.caretRangeFromPoint === 'function') {
      const range = doc.caretRangeFromPoint(x, y);
      if (range) ({ startContainer: node, startOffset: offset } = range);
    } else if (typeof doc.caretPositionFromPoint === 'function') {
      const position = doc.caretPositionFromPoint(x, y);
      if (position) ({ offsetNode: node, offset } = position);
    }
    if (node !== text) return null;

    offset = Math.min(offset, textarea.value.length);
    const caret = document.createRange();
    caret.setStart(text, offset);
    caret.collapse(true);
    const rects = caret.getClientRects();
    const r = rects.length ? rects[0] : caret.getBoundingClientRect();
    const lineHeight = parseFloat(style.lineHeight);
    return {
      offset,
      rect: { left: r.left, top: r.top, height: r.height || (Number.isNaN(lineHeight) ? 20 : lineHeight) },
    };
  } finally {
    mirror.remove();
  }
}

// ── Insertion ────────────────────────────────────────────────────────────────

/**
 * Replace `start..end` of the textarea with `text` as if typed: through
 * `insertText`, so Carta sees an `input` event and undo works, with a
 * `setRangeText` fallback.
 */
function insertIntoTextarea(textarea: HTMLTextAreaElement, start: number, end: number, text: string) {
  const length = textarea.value.length;
  start = Math.max(0, Math.min(start, length));
  end = Math.max(start, Math.min(end, length));
  textarea.focus({ preventScroll: true });
  textarea.setSelectionRange(start, end);
  if (!document.execCommand('insertText', false, text)) {
    textarea.setRangeText(text, start, end, 'end');
    textarea.dispatchEvent(new Event('input', { bubbles: true }));
  }
}

/** The part of a textarea `storeAndInsert` reads; a plain object in tests. */
export type TextareaLike = Pick<HTMLTextAreaElement, 'value' | 'selectionStart' | 'selectionEnd'>;

export interface StoreAndInsertDeps<T extends TextareaLike> {
  getTextarea: () => T | null | undefined;
  getPageKey: () => string | null;
  /** True once the editor was torn down. */
  isDisposed: () => boolean;
  insert: (textarea: T, start: number, end: number, text: string) => void;
  onInserted: () => void;
  onError: (message: string) => void;
}

/**
 * Store each item in order, then insert the references of those that
 * succeeded at `point`, moved past whatever was typed or inserted meanwhile
 * (or at the caret when that edit touched it). A failed item is reported by
 * name and adds no reference; nothing is inserted when the page changed.
 */
export async function storeAndInsert<T extends TextareaLike>(
  items: PendingAttachment[],
  point: InsertionPoint | null,
  deps: StoreAndInsertDeps<T>,
): Promise<void> {
  if (!items.length) return;
  const pageKey = deps.getPageKey();
  const references: string[] = [];
  const errors: string[] = [];
  for (const item of items) {
    try {
      const stored = await item.store();
      references.push(attachmentReference(stored.name, stored.uuid));
    } catch (e) {
      errors.push(`Could not attach ${item.name}: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  const textarea = deps.getTextarea();
  if (references.length) {
    if (deps.isDisposed() || !textarea || deps.getPageKey() !== pageKey) {
      errors.push(`Stored ${references.length} attachment(s), but the page changed before they could be linked`);
    } else {
      const range = point ? rebaseRange(point.value, textarea.value, point) : null;
      const start = range?.start ?? textarea.selectionStart;
      const end = range?.end ?? textarea.selectionEnd;
      deps.insert(textarea, start, end, references.join('\n'));
      deps.onInserted();
    }
  }
  if (errors.length) deps.onError(errors.join('\n'));
}

// ── Wiring ───────────────────────────────────────────────────────────────────

/** Wire drop and paste handling onto the editor; returns the cleanup. */
export function attachEditorAttachments(options: EditorAttachmentOptions): () => void {
  const { container } = options;
  let disposed = false;
  let internalDrag = false;
  let frame = 0;

  const inputArea = () => container.querySelector<HTMLElement>('.carta-input');

  function hideIndicator() {
    if (frame) cancelAnimationFrame(frame);
    frame = 0;
    options.onIndicator(null);
  }

  /** Draw the indicator at the would-be insertion point, at most once a frame. */
  function showIndicatorAt(x: number, y: number) {
    if (frame) return;
    frame = requestAnimationFrame(() => {
      frame = 0;
      const textarea = options.getTextarea();
      const hit = textarea ? textareaHitTest(textarea, x, y) : null;
      const area = inputArea()?.getBoundingClientRect();
      const origin = container.getBoundingClientRect();
      if (hit) {
        options.onIndicator({ left: hit.rect.left - origin.left, top: hit.rect.top - origin.top, height: hit.rect.height });
      } else if (area) {
        // Over the editor but not over text: the reference goes at the caret.
        options.onIndicator({ left: area.left - origin.left, top: area.top - origin.top, height: 0 });
      }
    });
  }

  function isOverInput(x: number, y: number): boolean {
    const area = inputArea()?.getBoundingClientRect();
    return !!area && x >= area.left && x <= area.right && y >= area.top && y <= area.bottom;
  }

  function storeAndInsertHere(items: PendingAttachment[], point: InsertionPoint | null) {
    return storeAndInsert(items, point, {
      getTextarea: options.getTextarea,
      getPageKey: options.getPageKey,
      isDisposed: () => disposed,
      insert: insertIntoTextarea,
      onInserted: options.onInserted,
      onError: options.onError,
    });
  }

  /** Insertion point at `offset`, or at the current selection without one. */
  function pointAt(offset?: number): InsertionPoint | null {
    const textarea = options.getTextarea();
    if (!textarea) return null;
    return offset === undefined
      ? { start: textarea.selectionStart, end: textarea.selectionEnd, value: textarea.value }
      : { start: offset, end: offset, value: textarea.value };
  }

  function pendingFromContent(content: DropContent, prefix: string): PendingAttachment[] {
    const now = new Date();
    const bytes = (name: string, data: () => Promise<Uint8Array>): PendingAttachment => ({
      name,
      store: async () => ({ uuid: await options.addBytes(name, await data()), name }),
    });
    const url = (href: string): PendingAttachment => {
      if (isWebUrl(href)) return { name: href, store: () => options.addUrl(href, fragmentName(undefined, '', prefix, now)) };
      // A `data:` image inside the dragged HTML: its bytes are right here.
      const data = decodeDataUrl(href);
      const name = fragmentName(undefined, data?.mime ?? '', prefix, now);
      return {
        name,
        store: async () => {
          if (!data) throw new Error('unreadable data URL');
          return { uuid: await options.addBytes(name, data.bytes), name };
        },
      };
    };
    switch (content.kind) {
      case 'paths':
        return content.paths.map((path) => {
          switch (nativeDropSource(path)) {
            case 'path':
              return { name: baseName(path), store: async () => ({ uuid: await options.addPath(path), name: baseName(path) }) };
            case 'url':
              return url(path);
            case 'unsupported':
              return {
                name: path,
                store: () => Promise.reject(new Error('only files and web links can be attached')),
              };
          }
        });
      case 'urls':
        return content.urls.map(url);
      case 'files':
        return content.files.map((file) =>
          bytes(fragmentName(file.name, file.type, prefix, now), async () => new Uint8Array(await file.arrayBuffer())),
        );
      case 'html':
        return [bytes(fragmentName(undefined, 'text/html', prefix, now), async () => new TextEncoder().encode(content.html))];
      case 'text':
        return [bytes(fragmentName(undefined, 'text/plain', prefix, now), async () => new TextEncoder().encode(content.text))];
      case 'none':
        return [];
    }
  }

  // ── OS file drops (Tauri native drag-drop: paths + position) ───────────────

  let unlistenNative: (() => void) | null = null;
  try {
    void getCurrentWebview()
      .onDragDropEvent((event) => {
        const payload = event.payload;
        if (payload.type === 'leave') {
          hideIndicator();
          return;
        }
        if (!options.isActive()) return;
        const { x, y } = dropPositionToClient(payload.position, window.devicePixelRatio, navigator.userAgent);
        if (payload.type === 'enter' || payload.type === 'over') {
          if (isOverInput(x, y)) showIndicatorAt(x, y);
          else hideIndicator();
          return;
        }
        // drop
        hideIndicator();
        if (!payload.paths.length || !isOverInput(x, y)) return;
        const textarea = options.getTextarea();
        const hit = textarea ? textareaHitTest(textarea, x, y) : null;
        void storeAndInsertHere(pendingFromContent({ kind: 'paths', paths: payload.paths }, 'dropped'), pointAt(hit?.offset));
      })
      .then((stop) => {
        if (disposed) stop();
        else unlistenNative = stop;
      })
      .catch((e) => console.warn('[editor-attachments] native drag-drop unavailable:', e));
  } catch (e) {
    // Outside Tauri (unit tests, plain browser) there is no webview to listen to.
    console.warn('[editor-attachments] native drag-drop unavailable:', e);
  }

  // ── Fragment drops (HTML5) ─────────────────────────────────────────────────

  function fromInput(target: EventTarget | null): boolean {
    return target instanceof Element && !!target.closest('.carta-input');
  }

  // Anything dragged out of Cerbo's own window (a selection in the editor, a
  // link in a side panel) is dropped by the browser as before, not stored;
  // only drags from other windows and applications become Attachments.
  function onDragStart() {
    internalDrag = true;
  }

  function onDragEnd() {
    internalDrag = false;
    hideIndicator();
  }

  function onDragOver(e: DragEvent) {
    if (internalDrag || !options.isActive() || !fromInput(e.target) || !e.dataTransfer) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = 'copy';
    showIndicatorAt(e.clientX, e.clientY);
  }

  function onDragLeave(e: DragEvent) {
    const next = e.relatedTarget as Node | null;
    if (!next || !container.contains(next)) hideIndicator();
  }

  function onDrop(e: DragEvent) {
    hideIndicator();
    const internal = internalDrag;
    // `dragend` does not fire when the source element went away mid-drag.
    internalDrag = false;
    if (internal || !options.isActive() || !fromInput(e.target) || !e.dataTransfer) return;
    const content = classifyDrop(e.dataTransfer);
    if (content.kind === 'none') return;
    e.preventDefault();
    e.stopPropagation();
    const textarea = options.getTextarea();
    const hit = textarea ? textareaHitTest(textarea, e.clientX, e.clientY) : null;
    void storeAndInsertHere(pendingFromContent(content, 'dropped'), pointAt(hit?.offset));
  }

  // ── Paste ──────────────────────────────────────────────────────────────────

  function onPaste(e: ClipboardEvent) {
    if (!options.isActive() || !fromInput(e.target) || !e.clipboardData) return;
    const content = classifyPaste(e.clipboardData);
    if (content.kind === 'none') {
      const text = e.clipboardData.getData('text/plain');
      // Text-only clipboards keep the default paste, unless the text is the
      // path list a file manager puts beside copied files that the webview
      // did not expose: then Rust reads the files from the clipboard.
      if (!looksLikeCopiedPaths(text)) return;
      e.preventDefault();
      e.stopPropagation();
      void pastePathsOrText(text, pointAt());
      return;
    }
    e.preventDefault();
    e.stopPropagation();
    void storeAndInsertHere(pendingFromContent(content, 'pasted'), pointAt());
  }

  async function pastePathsOrText(text: string, point: InsertionPoint | null) {
    let paths: string[] = [];
    try {
      paths = await options.readClipboardPaths();
    } catch (err) {
      console.warn('[editor-attachments] reading the clipboard failed:', err);
    }
    if (paths.length) {
      await storeAndInsertHere(pendingFromContent({ kind: 'paths', paths }, 'pasted'), point);
      return;
    }
    // Only text after all: paste it where the default paste would have.
    const textarea = options.getTextarea();
    if (disposed || !textarea) return;
    const range = point ? rebaseRange(point.value, textarea.value, point) : null;
    insertIntoTextarea(textarea, range?.start ?? textarea.selectionStart, range?.end ?? textarea.selectionEnd, text);
  }

  // Capture phase, so these run before Carta's own textarea handlers. Drag
  // start and end are watched on the whole document to tell in-app drags.
  document.addEventListener('dragstart', onDragStart, true);
  document.addEventListener('dragend', onDragEnd, true);
  container.addEventListener('dragover', onDragOver, true);
  container.addEventListener('dragleave', onDragLeave, true);
  container.addEventListener('drop', onDrop, true);
  container.addEventListener('paste', onPaste, true);

  return () => {
    disposed = true;
    hideIndicator();
    unlistenNative?.();
    document.removeEventListener('dragstart', onDragStart, true);
    document.removeEventListener('dragend', onDragEnd, true);
    container.removeEventListener('dragover', onDragOver, true);
    container.removeEventListener('dragleave', onDragLeave, true);
    container.removeEventListener('drop', onDrop, true);
    container.removeEventListener('paste', onPaste, true);
  };
}
