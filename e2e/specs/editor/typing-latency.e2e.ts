import { $, $$, browser, expect } from '@wdio/globals';
import { LARGE_PAGE_BYTES, LARGE_PAGE_UUID } from '../../fixtures/large-page.ts';

// Spec `editor-input-performance`: typing 137 characters into the 137 KB
// fixture page (and into a small page) loses no keystroke, each key updates
// the textarea, and the hidden preview stays unrendered while writing.
//
// Keys are sent one by one so each is checked before the next. The page also
// records keydown → next frame (rAF + a posted message, about when the frame
// is painted) per key. The numbers are printed, and asserted against the
// 16 ms p95 budget only with CERBO_LATENCY_BUDGET=1: the budget is defined on
// the reference machine, not on a virtual X display.

// Letters, digits and spaces only: no `[`, `(` or `:` that would open the link
// or emoji pickers.
const TYPED = 'cerbo keeps typing fast 137 '.repeat(5).slice(0, 137);

const INDEX_UUID = 'ba079793-b2ff-4ca6-ba88-8764ce6668c3';

const BUDGET_MS = 16;
const ASSERT_BUDGET = process.env.CERBO_LATENCY_BUDGET === '1';

/** Autosave debounce in PageEditor. */
const AUTOSAVE_MS = 800;

async function openPage(uuid: string) {
  const row = $(`.page-list .item.page[data-uuid="${uuid}"] .page-btn`);
  await row.waitForDisplayed({ timeout: 30_000 });
  // Right after start-up the list may re-render under the first click: click
  // again until the row is active.
  await browser.waitUntil(
    async () => {
      const active = await browser.execute(
        (id: string) => !!document.querySelector(`.page-list .item.page.active[data-uuid="${id}"]`),
        uuid,
      );
      if (!active) await row.click().catch(() => {});
      return active;
    },
    { timeout: 30_000, interval: 1_000, timeoutMsg: `page ${uuid} did not open` },
  );
}

async function enterWriteMode() {
  if (!(await $('.page-editor.write-mode').isExisting())) {
    await $('.mode-toggle-btn').click();
  }
  await $('.page-editor.write-mode .carta-input textarea').waitForExist({ timeout: 10_000 });
}

function textareaValue(): Promise<string> {
  return browser.execute(() => document.querySelector<HTMLTextAreaElement>('.carta-input textarea')?.value ?? '');
}

/** Focuses the textarea with the caret at `offset` and starts recording frame timings. */
async function prepareTyping(offset: number) {
  await browser.execute((at: number) => {
    const w = window as unknown as { __latency: number[]; __latencyInstalled?: boolean };
    w.__latency = [];
    if (!w.__latencyInstalled) {
      w.__latencyInstalled = true;
      window.addEventListener(
        'keydown',
        (e) => {
          const start = e.timeStamp;
          requestAnimationFrame(() => {
            const channel = new MessageChannel();
            channel.port1.onmessage = () => w.__latency.push(performance.now() - start);
            channel.port2.postMessage(0);
          });
        },
        true,
      );
    }
    const textarea = document.querySelector<HTMLTextAreaElement>('.carta-input textarea')!;
    textarea.focus();
    textarea.setSelectionRange(at, at);
  }, offset);
}

/**
 * Types TYPED key by key at `offset`, checking the textarea after every key.
 * With `pauseAfter`, waits past the autosave debounce after that many keys and
 * types the rest while the save runs.
 */
async function typeAndCheck(offset: number, pauseAfter?: number) {
  for (let i = 0; i < TYPED.length; i++) {
    if (i === pauseAfter) await browser.pause(AUTOSAVE_MS + 50);
    await browser.keys(TYPED[i]);
    const expected = TYPED.slice(0, i + 1);
    await browser.waitUntil(
      async () =>
        (await browser.execute(
          (at: number, len: number) =>
            document.querySelector<HTMLTextAreaElement>('.carta-input textarea')?.value.slice(at, at + len) ?? '',
          offset,
          i + 1,
        )) === expected,
      { timeout: 5_000, timeoutMsg: `key ${i + 1} (${JSON.stringify(TYPED[i])}) did not reach the textarea` },
    );
  }
}

async function reportLatency(label: string) {
  // The last key's frame callback may still be pending.
  await browser.waitUntil(
    async () => (await browser.execute(() => (window as unknown as { __latency: number[] }).__latency.length)) >= TYPED.length,
    { timeout: 5_000, timeoutMsg: 'not every key reached a frame' },
  );
  const samples = await browser.execute(() => (window as unknown as { __latency: number[] }).__latency.slice());
  const sorted = [...samples].sort((a, b) => a - b);
  const at = (q: number) => sorted[Math.min(sorted.length - 1, Math.floor(q * sorted.length))] ?? NaN;
  console.log(
    `[typing-latency] ${label}: ${samples.length} keys, keydown→frame p50 ${at(0.5).toFixed(1)} ms, ` +
      `p95 ${at(0.95).toFixed(1)} ms, max ${sorted.at(-1)?.toFixed(1)} ms`,
  );
  expect(samples.length).toBe(TYPED.length);
  if (ASSERT_BUDGET) expect(at(0.95)).toBeLessThanOrEqual(BUDGET_MS);
}

async function setPreviewMode() {
  if (await $('.page-editor.write-mode').isExisting()) await $('.mode-toggle-btn').click();
  await $('.carta-renderer').waitForExist({ timeout: 10_000 });
}

/** Puts the caret at `offset` in the focused textarea, scrolled into view. */
async function placeCaret(offset: number) {
  await browser.execute((at: number) => {
    const textarea = document.querySelector<HTMLTextAreaElement>('.carta-input textarea')!;
    textarea.focus();
    textarea.setSelectionRange(at, at);
    // Scroll the caret to about a third of the box (the fixture's lines are
    // uniform, so the offset share of the text is close to the scroll share).
    const share = at / Math.max(1, textarea.value.length);
    textarea.scrollTop = Math.max(0, share * textarea.scrollHeight - textarea.clientHeight / 3);
  }, offset);
}

/** Sends `text` one key at a time (a key array is one chord: `[[` would type one `[`). */
async function typeKeys(text: string) {
  for (const key of text) await browser.keys(key);
}

/**
 * Whether the popup is next to the visible textarea, with 200 px of slack for
 * Carta's own placement (it flips above the caret near the window bottom and
 * counts the textarea offset twice). A popup placed from the top of the text
 * instead of the scrolled view lands ~100 000 px away.
 */
async function insideTextarea(selector: string): Promise<boolean> {
  const { inside, detail } = await browser.execute((sel: string) => {
    const el = document.querySelector(sel);
    const textarea = document.querySelector<HTMLTextAreaElement>('.carta-input textarea');
    if (!el || !textarea) return { inside: false, detail: 'missing' };
    const a = el.getBoundingClientRect();
    const b = textarea.getBoundingClientRect();
    return {
      inside: a.height > 0 && a.bottom >= b.top - 200 && a.top <= b.bottom + 200,
      detail: `popup ${a.top}–${a.bottom}, textarea ${b.top}–${b.bottom}, window ${window.innerHeight}, scrollTop ${textarea.scrollTop}`,
    };
  }, selector);
  if (!inside) console.log(`[typing-latency] ${selector} outside the textarea: ${detail}`);
  return inside;
}

describe('typing latency', () => {
  it('types 137 characters into the 137 KB page without losing any', async () => {
    await openPage(LARGE_PAGE_UUID);
    await enterWriteMode();
    // Large pages use the plain textarea, without Carta's overlay.
    await $('.carta-editor[data-input-mode="plain"]').waitForExist({ timeout: 10_000 });

    const before = await textareaValue();
    expect(before.length).toBe(LARGE_PAGE_BYTES);
    const offset = before.indexOf('\n', before.length >> 1) + 1;

    // Autosave fires after the first 37 keys; the other 100 are typed while
    // it runs, and none may be lost.
    await prepareTyping(offset);
    await typeAndCheck(offset, 37);
    await reportLatency('137 KB page');

    const expected = before.slice(0, offset) + TYPED + before.slice(offset);
    expect(await textareaValue()).toBe(expected);
    // The second autosave, after typing stopped, keeps the text too.
    await browser.pause(2_000);
    expect(await textareaValue()).toBe(expected);
  });

  it('keeps pickers, undo and the toolbar in the plain textarea', async () => {
    await openPage(LARGE_PAGE_UUID);
    await enterWriteMode();
    await $('.carta-editor[data-input-mode="plain"]').waitForExist({ timeout: 10_000 });

    const before = await textareaValue();
    const offset = before.indexOf('\n', before.length >> 1) + 1;

    // `[[` opens the link picker next to the caret, deep in the page.
    await placeCaret(offset);
    await typeKeys('[[');
    await $('.wikilink-autocomplete').waitForDisplayed({ timeout: 5_000 });
    expect(await insideTextarea('.wikilink-autocomplete')).toBe(true);
    // `]` dismisses it (Escape would leave write mode).
    await typeKeys(']]');
    await $('.wikilink-autocomplete').waitForExist({ reverse: true, timeout: 5_000 });

    // `:smi` opens the emoji picker inside the visible box, not below it.
    await typeKeys(' :smi');
    await $('.carta-emoji').waitForDisplayed({ timeout: 5_000 });
    expect(await insideTextarea('.carta-emoji')).toBe(true);
    await browser.keys(' ');

    // Ctrl+Z goes back through Carta's history: a word typed after a pause
    // is taken out again.
    await browser.pause(1_000);
    const beforeWord = await textareaValue();
    await typeKeys('undo137');
    await browser.waitUntil(async () => (await textareaValue()) !== beforeWord, { timeout: 5_000 });
    await browser.pause(1_000);
    await browser.keys(['Control', 'z']);
    await browser.waitUntil(async () => (await textareaValue()) === beforeWord, {
      timeout: 5_000,
      timeoutMsg: 'Ctrl+Z did not remove the typed word',
    });

    // The Bold icon wraps the selection.
    await browser.execute((from: number) => {
      const textarea = document.querySelector<HTMLTextAreaElement>('.carta-input textarea')!;
      const at = textarea.value.indexOf('Paragraph', from);
      textarea.focus();
      textarea.setSelectionRange(at, at + 'Paragraph'.length);
    }, offset);
    await $('.carta-toolbar button[aria-label="Bold"]').click();
    await browser.waitUntil(async () => (await textareaValue()).includes('**Paragraph**'), {
      timeout: 5_000,
      timeoutMsg: 'Bold did not wrap the selection',
    });
  });

  it('types 137 characters into a small page and keeps the preview unrendered', async () => {
    await openPage(INDEX_UUID);
    await enterWriteMode();
    await expect($('.carta-editor[data-input-mode="plain"]')).not.toExist();

    // Type on the line after the H1: typing into the H1 would retitle the page.
    const before = await textareaValue();
    expect(before.endsWith('\n')).toBe(true);
    await prepareTyping(before.length);
    await typeAndCheck(before.length);
    await reportLatency('small page');
    expect(await textareaValue()).toBe(before + TYPED);

    // The hidden renderer of the write tab did no markdown work.
    const hiddenPreview = await browser.execute(() => document.querySelector('.carta-renderer')?.textContent ?? '');
    expect(hiddenPreview.trim()).toBe('');

    // Switching to preview renders the typed text.
    await $('.mode-toggle-btn').click();
    await browser.waitUntil(
      async () =>
        (await browser.execute(() => document.querySelector('.carta-renderer')?.textContent ?? '')).includes(
          TYPED.trim(),
        ),
      { timeout: 10_000, timeoutMsg: 'preview did not render the typed text' },
    );
  });

  it('keeps preview links working when opening a large page in preview', async () => {
    await openPage(INDEX_UUID);
    await setPreviewMode();
    await openPage(LARGE_PAGE_UUID);
    const selector = '.carta-renderer a[href^="cerbo://objects/ba079793"]';
    await $$(selector)[0].waitForExist({ timeout: 10_000 });
    const link = $$(selector)[0];
    await link.scrollIntoView();
    await link.click();
    await browser.waitUntil(
      async () => (await $('.page-list .item.page.active').getAttribute('data-uuid')) === INDEX_UUID,
      { timeout: 10_000, timeoutMsg: 'cerbo link in the large page preview did not open the index page' },
    );
  });

  it('switches a page that grows past the threshold to the plain textarea, keeping the caret', async () => {
    await openPage(INDEX_UUID);
    await enterWriteMode();
    await expect($('.carta-editor[data-input-mode="plain"]')).not.toExist();

    // A large paste: set the value as an input event would.
    const caret = await browser.execute(() => {
      const textarea = document.querySelector<HTMLTextAreaElement>('.carta-input textarea')!;
      const filler = 'pasted line to grow the page past the threshold\n'.repeat(1370);
      textarea.focus();
      textarea.setRangeText(filler, textarea.value.length, textarea.value.length, 'end');
      textarea.dispatchEvent(new Event('input', { bubbles: true }));
      return textarea.selectionStart;
    });
    await $('.carta-editor[data-input-mode="plain"]').waitForExist({ timeout: 10_000 });
    const after = await browser.execute(() => {
      const textarea = document.querySelector<HTMLTextAreaElement>('.carta-input textarea')!;
      return { caret: textarea.selectionStart, focused: document.activeElement === textarea };
    });
    expect(after).toEqual({ caret, focused: true });
  });
});
