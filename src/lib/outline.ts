/**
 * Pure helpers behind the Outline sub-panel of the right panel (change
 * `add-right-panel-tabs`, spec `desktop-outline-panel`, design D5). Free of
 * Svelte and Tauri so vitest runs them.
 */

/** One heading of a page: where it is in the text and how it reads. */
export interface Heading {
  /** 1 to 6, the number of `#`. */
  level: number;
  /** The heading text without the `#` marks and any closing `#`s. */
  text: string;
  /** Character offset of the start of the heading's line. */
  offset: number;
  /** Zero-based line number. */
  line: number;
}

const ATX = /^ {0,3}(#{1,6})[ \t]+(.*?)[ \t]*$/;
const FENCE = /^ {0,3}(`{3,}|~{3,})/;
/** `Title` underlined by `=====` (level 1) or `-----` (level 2). */
const SETEXT = /^ {0,3}(=+|-+)[ \t]*$/;
/** Lines that start something other than a paragraph: lists, quotes, rules. */
const NOT_PARAGRAPH = /^ {0,3}(?:[-*+>]|\d+[.)])(?:[ \t]|$)|^ {0,3}(?:(?:\*[ \t]*){3,}|(?:_[ \t]*){3,})$/;

/** `## Title ##` -> `Title`; a closing run needs a space before it. */
function stripClosing(text: string): string {
  return text.replace(/(^|[ \t]+)#+$/, '').trim();
}

/** Number of leading lines taken by YAML front matter (0 when there is none). */
function frontMatterLines(lines: readonly string[]): number {
  if (lines.length === 0 || lines[0].replace(/\r$/, '').trimEnd() !== '---') return 0;
  for (let i = 1; i < lines.length; i++) {
    const t = lines[i].replace(/\r$/, '').trimEnd();
    if (t === '---' || t === '...') return i + 1;
  }
  return 0;
}

/**
 * The headings of `markdown` in document order: `#` headings and setext
 * headings (`Title` over `====` or `----`). Headings inside fenced code blocks
 * and front matter are skipped, and so are empty ones (`#` alone).
 */
export function extractHeadings(markdown: string): Heading[] {
  const headings: Heading[] = [];
  const lines = markdown.split('\n');
  const skip = frontMatterLines(lines);
  let fence: string | null = null;
  // The paragraph being read, which a following underline turns into a heading.
  let para: { texts: string[]; offset: number; line: number } | null = null;
  let offset = 0;
  for (let line = 0; line < lines.length; line++) {
    const raw = lines[line];
    const lineOffset = offset;
    offset += raw.length + 1;
    if (line < skip) continue;
    const text = raw.endsWith('\r') ? raw.slice(0, -1) : raw;
    const open = FENCE.exec(text);
    if (fence) {
      // A closing fence uses the same character, at least as long, and nothing else.
      if (open && open[1][0] === fence[0] && open[1].length >= fence.length && text.trim() === open[1]) fence = null;
      continue;
    }
    if (open) {
      fence = open[1];
      para = null;
      continue;
    }
    if (text.trim() === '') {
      para = null;
      continue;
    }
    const underline = SETEXT.exec(text);
    if (underline && para) {
      const title = para.texts.join(' ');
      headings.push({ level: underline[1][0] === '=' ? 1 : 2, text: title, offset: para.offset, line: para.line });
      para = null;
      continue;
    }
    const m = ATX.exec(text);
    if (m) {
      const title = stripClosing(m[2]);
      if (title) headings.push({ level: m[1].length, text: title, offset: lineOffset, line });
      para = null;
      continue;
    }
    if (para) {
      if (NOT_PARAGRAPH.test(text)) para = null;
      else para.texts.push(text.trim());
    } else if (!NOT_PARAGRAPH.test(text) && !/^( {4}|\t)/.test(text) && !/^ {0,3}(?:-{3,}|={3,})/.test(text)) {
      para = { texts: [text.trim()], offset: lineOffset, line };
    }
  }
  return headings;
}

let memoText: string | null = null;
let memoHeadings: Heading[] = [];

/** `extractHeadings` that remembers its last answer, so a re-render costs nothing. */
export function headingsFor(markdown: string): Heading[] {
  if (memoText !== markdown) {
    memoHeadings = extractHeadings(markdown);
    memoText = markdown;
  }
  return memoHeadings;
}

/**
 * How deep each heading sits in the tree: the number of earlier headings it is
 * nested under. A page that starts at `##` still starts at depth 0.
 */
export function outlineDepths(headings: readonly Heading[]): number[] {
  const stack: number[] = [];
  return headings.map((h) => {
    while (stack.length > 0 && stack[stack.length - 1] >= h.level) stack.pop();
    stack.push(h.level);
    return stack.length - 1;
  });
}

/**
 * The index of the last heading whose top is at or above `edge`, or -1 when
 * the visible area starts above the first heading. `topOf(i)` must not
 * decrease with `i`; it is called O(log n) times.
 */
export function currentHeadingIndex(count: number, topOf: (index: number) => number, edge: number): number {
  let lo = 0;
  let hi = count - 1;
  let found = -1;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    if (topOf(mid) <= edge) {
      found = mid;
      lo = mid + 1;
    } else {
      hi = mid - 1;
    }
  }
  return found;
}

/** A heading as rendered in the preview. */
export interface RenderedHeading {
  level: number;
  text: string;
}

/** Heading text reduced to what survives rendering: no link syntax, no emphasis, no punctuation. */
export function comparableText(text: string): string {
  return text
    .replace(/!?\[([^\]]*)\]\([^)]*\)/g, '$1')
    .replace(/\[\[([^\]|]*\|)?([^\]]*)\]\]/g, '$2')
    .normalize('NFKC')
    .toLowerCase()
    .replace(/[^\p{L}\p{N}]+/gu, '');
}

/**
 * Pair the parsed headings with the rendered ones, as `{ heading, rendered }`
 * index pairs in document order. When the counts are equal the pairing is by
 * position; otherwise (raw HTML headings, headings the renderer drops) each
 * parsed heading takes the next rendered one of the same level and text, and
 * a heading with no such match is left out.
 */
export function pairRenderedHeadings(
  headings: readonly Heading[],
  rendered: readonly RenderedHeading[],
): { heading: number; rendered: number }[] {
  if (headings.length === rendered.length) return headings.map((_, i) => ({ heading: i, rendered: i }));
  const pairs: { heading: number; rendered: number }[] = [];
  const wanted = rendered.map((r) => comparableText(r.text));
  let next = 0;
  headings.forEach((h, i) => {
    const text = comparableText(h.text);
    for (let r = next; r < rendered.length; r++) {
      if (rendered[r].level === h.level && wanted[r] === text) {
        pairs.push({ heading: i, rendered: r });
        next = r + 1;
        return;
      }
    }
  });
  return pairs;
}
