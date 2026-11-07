/**
 * Pure logic for dropping and pasting into the editor (spec
 * `editor-attachment-drop`): what a drop or clipboard holds, what to name an
 * in-memory fragment, and the markdown reference to insert. The DOM and Tauri
 * wiring lives in `editor-attachments.ts`.
 */

/** Extensions the core stores with an `image/*` MIME type (`detect_mime_type`). */
const IMAGE_EXTENSIONS = ['png', 'jpg', 'jpeg', 'gif', 'webp', 'svg', 'bmp'];

/** File extension for the MIME types a fragment can arrive as. */
const EXTENSION_FOR_MIME: Record<string, string> = {
  'image/png': 'png',
  'image/jpeg': 'jpg',
  'image/gif': 'gif',
  'image/webp': 'webp',
  'image/svg+xml': 'svg',
  'image/bmp': 'bmp',
  'text/html': 'html',
  'text/plain': 'txt',
  'application/pdf': 'pdf',
};

/** The part of a drag the editor stores; the rest is left to the browser. */
export type DropContent =
  | { kind: 'files'; files: File[] }
  | { kind: 'paths'; paths: string[] }
  | { kind: 'urls'; urls: string[] }
  | { kind: 'html'; html: string }
  | { kind: 'text'; text: string }
  | { kind: 'none' };

/** The subset of `DataTransfer` the classifiers read; a fake in tests. */
export interface TransferLike {
  readonly types: readonly string[];
  readonly files?: ArrayLike<File> | null;
  getData(type: string): string;
}

function extensionOf(name: string): string {
  const dot = name.lastIndexOf('.');
  return dot > 0 ? name.slice(dot + 1).toLowerCase() : '';
}

/** True if a file of this name is stored as an image and inserted as `![…]`. */
export function isImageName(name: string): boolean {
  return IMAGE_EXTENSIONS.includes(extensionOf(name));
}

/** `![name](cerbo://objects/uuid)` for images, `[name](cerbo://objects/uuid)` otherwise. */
export function attachmentReference(name: string, uuid: string): string {
  const label = name.replace(/([\\[\]])/g, '\\$1');
  return `${isImageName(name) ? '!' : ''}[${label}](cerbo://objects/${uuid})`;
}

/** `20261107-131415` in local time, for names of unnamed fragments. */
function stamp(now: Date): string {
  const p = (n: number) => String(n).padStart(2, '0');
  return (
    `${now.getFullYear()}${p(now.getMonth() + 1)}${p(now.getDate())}-` +
    `${p(now.getHours())}${p(now.getMinutes())}${p(now.getSeconds())}`
  );
}

/**
 * Name to store a fragment under: the source's name when it has one, with the
 * extension its MIME type implies when the name lacks it, so the core detects
 * the same MIME type; otherwise `<prefix>-<stamp>.<ext>`.
 */
export function fragmentName(name: string | undefined, mime: string, prefix: string, now: Date): string {
  const ext = EXTENSION_FOR_MIME[mime.split(';')[0].trim().toLowerCase()] ?? '';
  const base = (name ?? '').split(/[\\/]/).pop()?.trim() ?? '';
  if (base && base !== '.' && base !== '..') {
    if (!ext || extensionOf(base) === ext || (ext === 'jpg' && extensionOf(base) === 'jpeg')) return base;
    return `${base}.${ext}`;
  }
  return ext ? `${prefix}-${stamp(now)}.${ext}` : `${prefix}-${stamp(now)}`;
}

/** Local path of a `file://` URI, or null for anything else. */
export function fileUriToPath(uri: string): string | null {
  const match = /^file:\/\/(?:localhost)?(\/[^?#]*)$/i.exec(uri.trim());
  if (!match) return null;
  try {
    return decodeURIComponent(match[1]);
  } catch {
    return null;
  }
}

/** Local paths in a `text/uri-list` (RFC 2483: one URI per line, `#` comments). */
export function parseUriList(text: string): string[] {
  return text
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line && !line.startsWith('#'))
    .map(fileUriToPath)
    .filter((path): path is string => path !== null);
}

/** Local paths in `x-special/gnome-copied-files`: `copy`/`cut`, then one URI per line. */
export function parseGnomeCopiedFiles(text: string): string[] {
  const [first, ...rest] = text.split(/\r?\n/);
  const lines = first === 'copy' || first === 'cut' ? rest : [first, ...rest];
  return parseUriList(lines.join('\n'));
}

function filesOf(transfer: TransferLike): File[] {
  return transfer.files ? Array.from(transfer.files) : [];
}

function copiedPaths(transfer: TransferLike): string[] {
  if (transfer.types.includes('x-special/gnome-copied-files')) {
    const paths = parseGnomeCopiedFiles(transfer.getData('x-special/gnome-copied-files'));
    if (paths.length) return paths;
  }
  if (transfer.types.includes('text/uri-list')) {
    return parseUriList(transfer.getData('text/uri-list'));
  }
  return [];
}

/** True for `http:`/`https:` URLs, which Rust downloads (`attachment_fetch`). */
export function isWebUrl(url: string): boolean {
  return /^https?:\/\/[^\s]+$/i.test(url.trim());
}

/** True for a base64 or URL-encoded `data:image/…` URL. */
export function isImageDataUrl(url: string): boolean {
  return /^data:image\/[\w.+-]+(;[^,]*)?,/i.test(url.trim());
}

/**
 * The image a browser drag carries when it offers no file: the `src` of HTML
 * that is a single `<img>` (possibly wrapped in a link or the `<meta>`/`<html>`
 * scaffolding browsers add), else a `text/uri-list` web URL whose path names
 * an image. Null when the drag is not an image.
 */
export function draggedImageUrl(transfer: TransferLike): string | null {
  const html = transfer.types.includes('text/html') ? transfer.getData('text/html') : '';
  if (html.trim()) {
    const rest = html
      .replace(/<!--[\s\S]*?-->/g, '')
      .replace(/<\/?(?:meta|html|head|body|a)\b[^>]*>/gi, '');
    const images = rest.match(/<img\b[^>]*>/gi) ?? [];
    if (images.length === 1 && !rest.replace(images[0], '').trim()) {
      const src = /\ssrc\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s>]+))/i.exec(images[0]);
      const url = decodeHtmlEntities(src?.[1] ?? src?.[2] ?? src?.[3] ?? '');
      if (isWebUrl(url) || isImageDataUrl(url)) return url.trim();
    }
    return null;
  }
  if (transfer.types.includes('text/uri-list')) {
    const first = transfer
      .getData('text/uri-list')
      .split(/\r?\n/)
      .map((line) => line.trim())
      .find((line) => line && !line.startsWith('#'));
    if (first && isWebUrl(first)) {
      const path = first.replace(/[?#].*$/, '');
      if (isImageName(path)) return first;
    }
  }
  return null;
}

/** The few entities that appear in attribute values browsers serialize. */
function decodeHtmlEntities(text: string): string {
  return text
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&amp;/g, '&');
}

/**
 * What an HTML5 drop on the editor stores, in preference order: files (in the
 * order they were dropped) → an image dragged out of a web page (by URL) →
 * `text/html` → `text/plain`.
 */
export function classifyDrop(transfer: TransferLike): DropContent {
  const files = filesOf(transfer);
  if (files.length) return { kind: 'files', files };
  const image = draggedImageUrl(transfer);
  if (image) return { kind: 'urls', urls: [image] };
  const html = transfer.types.includes('text/html') ? transfer.getData('text/html') : '';
  if (html.trim()) return { kind: 'html', html };
  const text = transfer.types.includes('text/plain') ? transfer.getData('text/plain') : '';
  if (text) return { kind: 'text', text };
  return { kind: 'none' };
}

/**
 * What a paste into the editor stores. Files copied in a file manager
 * (`file://` URIs) are copied by path; image data wins over the HTML or text a
 * program offers beside it; other file data is uploaded; text-only clipboards
 * yield `none` so the default paste inserts the text.
 */
export function classifyPaste(transfer: TransferLike): DropContent {
  const paths = copiedPaths(transfer);
  if (paths.length) return { kind: 'paths', paths };
  const files = filesOf(transfer);
  const images = files.filter((f) => f.type.startsWith('image/'));
  if (images.length) return { kind: 'files', files: images };
  if (files.length) return { kind: 'files', files };
  return { kind: 'none' };
}

/**
 * True when pasted text is nothing but absolute paths or `file://` URIs, one
 * per line: what file managers put beside the copied files. Such a paste asks
 * Rust for the clipboard's files (`clipboard_file_paths`) before falling back
 * to the text, in case the webview did not expose the file types.
 */
export function looksLikeCopiedPaths(text: string): boolean {
  const lines = text.split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
  return lines.length > 0 && lines.every((line) => line.startsWith('/') || fileUriToPath(line) !== null);
}

/** Last path component, `/` or `\` separated. */
export function baseName(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).pop() ?? path;
}

/**
 * The file an Attachment object stores, given its title: the title itself,
 * except names the object's own files use, which the core prefixes.
 */
export function attachmentFileName(title: string): string {
  return title === 'meta.ttl' || title === 'page.md' ? `attachment-${title}` : title;
}

/** Absolute path of an Attachment object's file inside the vault. */
export function attachmentFilePath(vaultPath: string, uuid: string, title: string): string {
  const base = vaultPath.replace(/[\\/]+$/, '');
  return `${base}/.cerbo/objects/${uuid}/${attachmentFileName(title)}`;
}

/** UUID of a `cerbo://objects/<uuid>` (or `cerbo://<uuid>`) URL, else null. */
export function cerboObjectUuid(url: string): string | null {
  const match = /^cerbo:\/\/(?:objects\/)?([0-9a-f-]{8,})\/?$/i.exec(url.trim());
  return match ? match[1] : null;
}

/**
 * Tauri drag-drop position → CSS client pixels. Tauri labels the position
 * physical, but on Linux wry passes GTK's widget coordinates through, which
 * are already logical; elsewhere they are physical and divided by the ratio.
 */
export function dropPositionToClient(
  position: { x: number; y: number },
  devicePixelRatio: number,
  userAgent: string,
): { x: number; y: number } {
  const linux = /Linux/i.test(userAgent) && !/Android/i.test(userAgent);
  const ratio = !linux && devicePixelRatio > 0 ? devicePixelRatio : 1;
  return { x: position.x / ratio, y: position.y / ratio };
}

/**
 * How a "path" from a native drop is stored: a local absolute path is copied
 * (`attachment_add`); a web URL, which wry reports as a path when a browser
 * image or link is dropped, is downloaded; anything else cannot be attached.
 */
export function nativeDropSource(path: string): 'path' | 'url' | 'unsupported' {
  if (isWebUrl(path)) return 'url';
  if (path.startsWith('/') || /^[a-z]:[\\/]/i.test(path) || path.startsWith('\\\\')) return 'path';
  return 'unsupported';
}

/** MIME type and bytes of a `data:` URL, or null when it is malformed. */
export function decodeDataUrl(url: string): { mime: string; bytes: Uint8Array } | null {
  const match = /^data:([^;,]*)((?:;[^;,]*)*),(.*)$/is.exec(url.trim());
  if (!match) return null;
  const mime = match[1] || 'text/plain';
  try {
    if (/;base64/i.test(match[2])) {
      const binary = atob(match[3].replace(/\s+/g, ''));
      return { mime, bytes: Uint8Array.from(binary, (c) => c.charCodeAt(0)) };
    }
    return { mime, bytes: new TextEncoder().encode(decodeURIComponent(match[3])) };
  } catch {
    return null;
  }
}

/**
 * Carry a selection range taken from `before` over to `after`, the textarea's
 * value once an upload finished. The edit is located by the longest common
 * prefix and suffix: edits wholly after the range leave it, edits wholly
 * before shift it; an edit that touches it yields null (use the caret).
 */
export function rebaseRange(
  before: string,
  after: string,
  range: { start: number; end: number },
): { start: number; end: number } | null {
  if (before === after) return range;
  const max = Math.min(before.length, after.length);
  let prefix = 0;
  while (prefix < max && before[prefix] === after[prefix]) prefix++;
  let suffix = 0;
  while (
    suffix < max - prefix &&
    before[before.length - 1 - suffix] === after[after.length - 1 - suffix]
  ) {
    suffix++;
  }
  // `before[prefix .. editEnd]` was replaced; an edit starting at the range's
  // end (typing on at the drop point) leaves the range where it was.
  const editEnd = before.length - suffix;
  if (prefix >= range.end) return range;
  if (editEnd <= range.start) {
    const delta = after.length - before.length;
    return { start: range.start + delta, end: range.end + delta };
  }
  return null;
}
