// Per-keystroke cost of Carta's speculative overlay update, outside the
// webview (spec `editor-input-performance`, LARGE_DOCUMENT_CHARS in
// src/lib/editor-performance.ts). Carta's `Input` calls `diffChars(previous,
// next)` on every key; this times one inserted character in the middle of the
// fixture page at several sizes. bun runs JavaScriptCore, the engine of
// WebKitGTK, so the numbers are a lower bound for the webview (no DOM work).
//
//   bun scripts/bench-overlay-diff.ts
import { diffChars } from 'diff';
import { largePageMarkdown } from '../e2e/fixtures/large-page.ts';

const SIZES_KB = [10, 25, 50, 75, 100, 137];
const KEYS = 137;

// Warm the JIT up first, so the first size is not charged for it.
for (let i = 0; i < 20; i++) diffChars(largePageMarkdown(10 * 1024), largePageMarkdown(10 * 1024) + 'x');

for (const kb of SIZES_KB) {
  const text = largePageMarkdown(kb * 1024);
  const offset = text.indexOf('\n', text.length >> 1) + 1;
  const times: number[] = [];
  let previous = text;
  for (let i = 0; i < KEYS; i++) {
    const next = previous.slice(0, offset + i) + 'x' + previous.slice(offset + i);
    const start = performance.now();
    diffChars(previous, next);
    times.push(performance.now() - start);
    previous = next;
  }
  times.sort((a, b) => a - b);
  const p50 = times[Math.floor(KEYS * 0.5)];
  const p95 = times[Math.floor(KEYS * 0.95)];
  console.log(`${String(kb).padStart(3)} KB  diffChars per key: p50 ${p50.toFixed(1)} ms, p95 ${p95.toFixed(1)} ms`);
}
