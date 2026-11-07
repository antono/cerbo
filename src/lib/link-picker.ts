/**
 * Pure logic behind the editor link picker (`WikilinkAutocomplete.svelte`,
 * design Decision 2): trigger detection on the text before the caret, and a
 * precomputed search index over vault objects.
 *
 * Per keystroke only the current line is scanned. The one exception is the
 * fenced-code check, which needs the lines above; it runs only when a trigger
 * candidate is found, and its answer is remembered per line (`FenceMemo`) so
 * typing on along that line does not repeat it.
 */

// ── Trigger detection ─────────────────────────────────────────────────────────

export type LinkTriggerKind =
  /** A single `[`: the picker replaces `[` + query with a full link. */
  | 'link'
  /** `[[`: same insertion, kept for the old wikilink habit. */
  | 'wikilink'
  /** `](cerbo://…`: the picker replaces the URL part only. */
  | 'cerbo-link';

export interface LinkTrigger {
  kind: LinkTriggerKind;
  /** Offset where the replacement starts (the `[`, or the `c` of `cerbo://`). */
  start: number;
  /** Text typed after the trigger, up to the caret. */
  query: string;
}

/**
 * The last fenced-code answer, kept by the caller between keystrokes. It holds
 * for one line only: a lookup on another line drops it, so an edit above the
 * line (which needs the caret there first) cannot leave a stale answer.
 */
export interface FenceMemo {
  last: { lineStart: number; inFence: boolean } | null;
}

export function createFenceMemo(): FenceMemo {
  return { last: null };
}

export interface DetectOptions {
  /** Remembers the fenced-code check for the caret's line across calls. */
  fenceMemo?: FenceMemo;
}

const WIKILINK = /\[\[([^\][\n]*)$/;
const CERBO_LINK = /\[([^\]]*)\]\(cerbo:\/\/([^)]*)\)$/;
const SINGLE = /\[([^[\]]*)$/;
/** Line prefix that makes a following `[` a task-list checkbox. */
const LIST_MARKER = /^\s*(?:>\s*)*(?:[-*+]|\d+[.)])\s+$/;

/**
 * The link trigger ending at `caret` in `text`, or null when the picker must
 * stay closed.
 */
export function detectTrigger(text: string, caret: number, options: DetectOptions = {}): LinkTrigger | null {
  const lineStart = text.lastIndexOf('\n', caret - 1) + 1;
  const line = text.slice(lineStart, caret);

  const memo = options.fenceMemo;
  if (memo?.last && memo.last.lineStart !== lineStart) memo.last = null;

  const trigger = matchLine(line, lineStart);
  if (!trigger) return null;

  // For `](cerbo://` the code check looks at the `](`, which ends the link text.
  const at = trigger.start - lineStart - (trigger.kind === 'cerbo-link' ? 2 : 0);
  if (inInlineCode(line, at)) return null;
  if (isFenceLine(line)) return null;
  if (inFencedBlockMemo(text, lineStart, memo)) return null;
  return trigger;
}

function inFencedBlockMemo(text: string, lineStart: number, memo: FenceMemo | undefined): boolean {
  if (memo?.last) return memo.last.inFence;
  const inFence = inFencedBlock(text, lineStart);
  if (memo) memo.last = { lineStart, inFence };
  return inFence;
}

function matchLine(line: string, lineStart: number): LinkTrigger | null {
  const wiki = WIKILINK.exec(line);
  if (wiki) return { kind: 'wikilink', start: lineStart + wiki.index, query: wiki[1] };

  const cerbo = CERBO_LINK.exec(line);
  if (cerbo) {
    // Just after `](`: the replacement covers `cerbo://<query>)`. Counted from
    // the match, which may begin at an earlier unclosed `[` on the line.
    const start = lineStart + cerbo.index + cerbo[1].length + 3;
    return { kind: 'cerbo-link', start, query: cerbo[2] };
  }

  const single = SINGLE.exec(line);
  if (!single) return null;
  const at = single.index;
  const query = single[1];
  // `![` is an image, `\[` a literal bracket (but `\\[` is a backslash and a bracket).
  if (line[at - 1] === '!' || isEscaped(line, at)) return null;
  // `[` followed by a space is plain text, like `[ ]` in prose.
  if (/^\s/.test(query)) return null;
  if (LIST_MARKER.test(line.slice(0, at))) return null;
  return { kind: 'link', start: lineStart + at, query };
}

/** True when `line[at]` follows an odd number of backslashes. */
function isEscaped(line: string, at: number): boolean {
  let n = 0;
  while (at - n > 0 && line[at - n - 1] === '\\') n++;
  return n % 2 === 1;
}

/** True when offset `at` of `line` is inside an inline code span. */
function inInlineCode(line: string, at: number): boolean {
  let open = 0;
  let i = 0;
  while (i < at) {
    if (line[i] !== '`') {
      i++;
      continue;
    }
    let run = 0;
    while (i < at && line[i] === '`') {
      run++;
      i++;
    }
    if (open === 0) open = run;
    else if (run === open) open = 0;
  }
  return open > 0;
}

/** A fence line: up to three spaces, then three or more backticks or tildes. */
function fenceOf(line: string): string | null {
  const m = /^ {0,3}(`{3,}|~{3,})/.exec(line);
  return m ? m[1] : null;
}

/** A closing fence carries no info string, only trailing whitespace. */
function isClosingFence(line: string, open: string): boolean {
  const m = /^ {0,3}(`{3,}|~{3,})[ \t]*$/.exec(line);
  return m !== null && m[1][0] === open[0] && m[1].length >= open.length;
}

function isFenceLine(line: string): boolean {
  return fenceOf(line) !== null;
}

/** True when the line starting at `lineStart` sits inside a fenced code block. */
function inFencedBlock(text: string, lineStart: number): boolean {
  let open: string | null = null;
  let i = 0;
  while (i < lineStart) {
    const end = text.indexOf('\n', i);
    const next = end === -1 || end >= lineStart ? lineStart : end + 1;
    // Cheap pre-check before the regex: fences start with ` or ~ after ≤ 3 spaces.
    let j = i;
    while (j < next && j - i < 3 && text[j] === ' ') j++;
    if (text[j] === '`' || text[j] === '~') {
      const fenceLine = text.slice(i, next).replace(/\n$/, '');
      if (open === null) open = fenceOf(fenceLine);
      else if (isClosingFence(fenceLine, open)) open = null;
    }
    i = next;
  }
  return open !== null;
}

// ── Insertion ─────────────────────────────────────────────────────────────────

export interface LinkTarget {
  uuid: string;
  title: string;
}

/** Brackets in a title would end the link text early. */
function escapeLinkText(title: string): string {
  return title.replace(/[[\]\\]/g, (c) => `\\${c}`);
}

/** The text that replaces `trigger.start .. caret` when `target` is chosen. */
export function insertionFor(kind: LinkTriggerKind, target: LinkTarget): string {
  if (kind === 'cerbo-link') return `cerbo://objects/${target.uuid})`;
  return `[${escapeLinkText(target.title)}](cerbo://objects/${target.uuid})`;
}

// ── Search index ──────────────────────────────────────────────────────────────

export interface LinkCandidate {
  uuid: string;
  title: string;
  objectType: string;
  /** The page's virtual path (pages only); null at the root or for non-pages. */
  virtualPath: string | null;
}

interface Entry {
  candidate: LinkCandidate;
  /** Lowercase title, for prefix-first ordering. */
  title: string;
  /** Lowercase `title \0 type \0 virtualPath`; the NUL keeps matches inside one field. */
  key: string;
}

export interface LinkIndex {
  /** Entries sorted by title. */
  entries: Entry[];
}

export const RESULT_LIMIT = 50;

/**
 * Build the search index once per change of the object list. `virtualPathOf`
 * supplies page paths, which the object list does not carry.
 */
export function buildLinkIndex(
  objects: readonly { uuid: string; title: string; objectType: string }[],
  virtualPathOf: (uuid: string) => string | null | undefined = () => null,
): LinkIndex {
  const entries: Entry[] = objects.map((o) => {
    const virtualPath = virtualPathOf(o.uuid) ?? null;
    const title = o.title.toLowerCase();
    return {
      candidate: { uuid: o.uuid, title: o.title, objectType: o.objectType, virtualPath },
      title,
      key: `${title}\0${o.objectType.toLowerCase()}\0${(virtualPath ?? '').toLowerCase()}`,
    };
  });
  // Plain code-unit order of the lowercase titles: an Intl.Collator sort costs
  // several times more on a large vault.
  entries.sort((a, b) => compare(a.title, b.title) || compare(a.candidate.title, b.candidate.title));
  return { entries };
}

function compare(a: string, b: string): number {
  return a < b ? -1 : a > b ? 1 : 0;
}

/**
 * Objects matching `query` in title, type or virtual path (case-insensitive),
 * titles starting with the query first, each group in title order.
 */
export function searchLinkIndex(index: LinkIndex, query: string, limit = RESULT_LIMIT): LinkCandidate[] {
  const { entries } = index;
  if (!query) return entries.slice(0, limit).map((e) => e.candidate);

  const q = query.toLowerCase();
  const prefix: LinkCandidate[] = [];
  const rest: LinkCandidate[] = [];
  for (const e of entries) {
    if (e.title.startsWith(q)) {
      prefix.push(e.candidate);
      if (prefix.length === limit) break;
    } else if (rest.length < limit && e.key.includes(q)) {
      rest.push(e.candidate);
    }
  }
  return prefix.concat(rest).slice(0, limit);
}

// ── Selection ─────────────────────────────────────────────────────────────────

/** The selection after ArrowDown (+1) or ArrowUp (-1), wrapping around `count` rows. */
export function moveSelection(selected: number, step: 1 | -1, count: number): number {
  if (count <= 0) return 0;
  return (clampSelection(selected, count) + step + count) % count;
}

/** `selected` kept inside a list of `count` rows, which may have shrunk since. */
export function clampSelection(selected: number, count: number): number {
  return Math.max(0, Math.min(selected, count - 1));
}
