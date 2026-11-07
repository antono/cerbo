import { $, $$, browser, expect } from '@wdio/globals';

// Spec `editor-position-restore`: the caret and scroll of the write view come
// back to where editing a page ended, after opening another page, after
// leaving write mode with Esc, and after an app restart.
//
// The page text is set through the DOM and the caret placed with
// `setSelectionRange`, then a real key press makes the edit at that line.
// Every step that places the caret first waits for the editor's own restore
// (`data-caret-restored`), which would otherwise move the caret again.

const LINE = 137;
const ESC_LINE = 60;
const SECOND = 'Cursor Second';

/** Click the sidebar row of the page titled `title`; returns its uuid. */
async function openRow(title: string): Promise<string> {
  const uuid = await browser.execute(
    (t) =>
      Array.from(document.querySelectorAll('.page-list .item.page'))
        .find((li) => li.querySelector('.label')?.textContent === t)
        ?.getAttribute('data-uuid'),
    title,
  );
  if (!uuid) throw new Error(`no page row "${title}"`);
  await $(`.page-list .item.page[data-uuid="${uuid}"] .page-btn`).click();
  return uuid;
}

/** The write view's text once it starts with `# title`. */
async function waitForEditorOf(title: string) {
  await browser.waitUntil(
    () =>
      browser.execute(
        (t) => (document.querySelector('.carta-input textarea') as HTMLTextAreaElement | null)?.value.startsWith(`# ${t}`) ?? false,
        title,
      ),
    { timeout: 10_000, timeoutMsg: `write view did not show "${title}"` },
  );
}

/** Wait until the editor has restored the caret of page `uuid`. */
async function waitForRestore(uuid: string) {
  await $(`.page-editor[data-caret-restored="${uuid}"]`).waitForExist({
    timeout: 10_000,
    timeoutMsg: `caret of ${uuid} was not restored`,
  });
}

/** 1-based line of the write view's caret, and whether the textarea has focus. */
function caret(): Promise<{ line: number; focused: boolean }> {
  return browser.execute(() => {
    const t = document.querySelector('.carta-input textarea') as HTMLTextAreaElement;
    return { line: t.value.slice(0, t.selectionStart).split('\n').length, focused: document.activeElement === t };
  });
}

function scrollTop(): Promise<number> {
  return browser.execute(() => (document.querySelector('.carta-input') as HTMLElement).scrollTop);
}

/** Wait for the caret on `line` with the textarea focused. */
async function waitForCaretOn(line: number) {
  let last = { line: 0, focused: false };
  await browser
    .waitUntil(async () => {
      last = await caret();
      return last.line === line && last.focused;
    }, { timeout: 10_000 })
    .catch(() => {
      throw new Error(`caret was on line ${last.line} (focused: ${last.focused}), expected line ${line}`);
    });
}

describe('caret restore', () => {
  let indexUuid = '';
  let leftAt = 0;

  it('opens a second page in write mode', async () => {
    await $('.page-list .page-btn').waitForDisplayed({ timeout: 30_000 });
    await browser.keys(['Control', 'n']);
    const input = $('#new-page-title');
    await input.waitForDisplayed({ timeout: 10_000 });
    await input.setValue(SECOND);
    await browser.keys('Enter');
    await input.waitForDisplayed({ reverse: true, timeout: 10_000 });
    await waitForEditorOf(SECOND);
  });

  it(`edits Index at line ${LINE}, scrolled down`, async () => {
    indexUuid = await openRow('Index');
    await waitForEditorOf('Index');
    await waitForRestore(indexUuid);
    await browser.execute((line) => {
      const t = document.querySelector('.carta-input textarea') as HTMLTextAreaElement;
      const body = Array.from({ length: 200 }, (_, i) => `line ${i + 2}`);
      t.value = ['# Index', ...body].join('\n');
      t.dispatchEvent(new Event('input', { bubbles: true }));
      const offset = t.value.indexOf(`line ${line}\n`) + 2;
      t.focus();
      t.setSelectionRange(offset, offset);
    }, LINE);
    await browser.keys('X');
    await browser.waitUntil(async () => (await caret()).line === LINE, { timeout: 5_000 });
    // Keep the caret line about 100 px below the top of the view.
    leftAt = await browser.execute((line) => {
      const scroller = document.querySelector('.carta-input') as HTMLElement;
      const t = scroller.querySelector('textarea') as HTMLTextAreaElement;
      const rowHeight = parseFloat(getComputedStyle(t).lineHeight) || 20;
      scroller.scrollTop = Math.max(0, (line - 1) * rowHeight - 100);
      return scroller.scrollTop;
    }, LINE);
    expect(leftAt).toBeGreaterThan(0);
    // Let the page autosave (800 ms) land: leaving sooner drops the edit.
    await browser.pause(1500);
  });

  it('returns the caret and scroll after visiting another page', async () => {
    const secondUuid = await openRow(SECOND);
    await waitForEditorOf(SECOND);
    await waitForRestore(secondUuid);
    await openRow('Index');
    await waitForEditorOf('Index');
    await waitForRestore(indexUuid);
    await waitForCaretOn(LINE);
    expect(Math.abs((await scrollTop()) - leftAt)).toBeLessThanOrEqual(2);
  });

  it('returns the caret after leaving write mode with Esc', async () => {
    await browser.execute((line) => {
      const t = document.querySelector('.carta-input textarea') as HTMLTextAreaElement;
      const offset = t.value.indexOf(`line ${line}\n`);
      t.setSelectionRange(offset, offset);
    }, ESC_LINE);
    await browser.keys('Escape');
    await $('.page-editor.write-mode').waitForExist({ reverse: true, timeout: 10_000 });
    await browser.keys('i');
    await waitForEditorOf('Index');
    await waitForRestore(indexUuid);
    await waitForCaretOn(ESC_LINE);
  });

  it('returns the caret after an app restart', async () => {
    await browser.keys('Escape');
    await $('.page-editor.write-mode').waitForExist({ reverse: true, timeout: 10_000 });
    // The save on Esc is on its way to cursor.toml.
    await browser.pause(1500);
    // Same launcher, so the same config dir: a fresh app on the same state.
    await browser.reloadSession();
    await $$('.page-list .page-btn')[0].waitForDisplayed({ timeout: 30_000 });
    await openRow('Index');
    await $('.page-editor:not(.write-mode)').waitForExist({ timeout: 10_000 });
    await browser.keys('i');
    await waitForEditorOf('Index');
    await waitForRestore(indexUuid);
    await waitForCaretOn(ESC_LINE);
  });
});
