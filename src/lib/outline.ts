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

/** `## Title ##` -> `Title`; a closing run needs a space before it. */
function stripClosing(text: string): string {
  return text.replace(/(^|[ \t]+)#+$/, '').trim();
}

/**
 * The headings of `markdown` in document order. Headings inside fenced code
 * blocks are skipped, and so are empty ones (`#` alone).
 */
export function extractHeadings(markdown: string): Heading[] {
  const headings: Heading[] = [];
  let fence: string | null = null;
  let offset = 0;
  let line = 0;
  for (const raw of markdown.split('\n')) {
    const text = raw.endsWith('\r') ? raw.slice(0, -1) : raw;
    const open = FENCE.exec(text);
    if (fence) {
      // A closing fence uses the same character, at least as long, and nothing else.
      if (open && open[1][0] === fence[0] && open[1].length >= fence.length && text.trim() === open[1]) fence = null;
    } else if (open) {
      fence = open[1];
    } else {
      const m = ATX.exec(text);
      if (m) {
        const title = stripClosing(m[2]);
        if (title) headings.push({ level: m[1].length, text: title, offset, line });
      }
    }
    offset += raw.length + 1;
    line += 1;
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
