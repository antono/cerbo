import { $, browser, expect } from '@wdio/globals';

// Spec `cerbo-link-autocomplete`: the `[` link picker in write mode. Typing
// goes through real key events so the picker sees the same `input` and
// `keydown` events as with a user. The fixture vault holds one object, the
// "Index" page, which is also the page being edited.
//
// WebKitWebDriver reports no rendered text, so the assertions read the DOM.

const INDEX_UUID = 'ba079793-b2ff-4ca6-ba88-8764ce6668c3';
const INDEX_LINK = `[Index](cerbo://objects/${INDEX_UUID})`;

const textarea = () => $('.carta-input textarea');
const picker = () => $('.wikilink-autocomplete');

/** Textarea value and caret offset. */
function editorState(): Promise<{ value: string; caret: number }> {
  return browser.execute(() => {
    const ta = document.querySelector<HTMLTextAreaElement>('.carta-input textarea')!;
    return { value: ta.value, caret: ta.selectionStart };
  });
}

/** Replace the page text with `text` through the editor and put the caret at the end. */
async function resetText(text: string) {
  await browser.execute((t) => {
    const ta = document.querySelector<HTMLTextAreaElement>('.carta-input textarea')!;
    ta.focus();
    ta.select();
    document.execCommand('insertText', false, t);
    ta.setSelectionRange(ta.value.length, ta.value.length);
  }, text);
}

async function type(text: string) {
  await browser.keys(text.split(''));
}

async function expectClosed() {
  // Give a wrongly opened picker a frame to appear before asserting.
  await browser.pause(150);
  await expect(picker()).not.toExist();
}

describe('link picker on [', () => {
  before(async () => {
    const page = $('.page-list .page-btn');
    await page.waitForDisplayed({ timeout: 30_000 });
    await browser.execute(() => document.querySelector<HTMLElement>('.page-list .page-btn')!.click());
    // `i` enters write mode when no input has focus.
    await browser.execute(() => (document.activeElement as HTMLElement | null)?.blur());
    await browser.keys('i');
    await textarea().waitForExist({ timeout: 10_000 });
    await browser.waitUntil(
      () => browser.execute(() => document.activeElement?.matches('.carta-input textarea') ?? false),
      { timeout: 10_000, timeoutMsg: 'editor textarea never got focus' },
    );
  });

  it('opens with a search field mirroring the query and inserts on Enter', async () => {
    await resetText('See ');
    await type('[Ind');
    await picker().waitForExist({ timeout: 5_000 });
    await expect($('.link-picker-query')).toHaveElementProperty('value', 'Ind');
    const titles = await browser.execute(() =>
      Array.from(document.querySelectorAll('.wikilink-autocomplete-item .object-title'), (e) => e.textContent),
    );
    expect(titles).toEqual(['Index']);
    // Focus stays in the editor while the picker is open.
    expect(await browser.execute(() => document.activeElement?.tagName)).toBe('TEXTAREA');

    await browser.keys('Enter');
    await expect(picker()).not.toExist();
    const { value, caret } = await editorState();
    expect(value).toBe(`See ${INDEX_LINK}`);
    expect(caret).toBe(value.length);
  });

  it('inserts on Tab', async () => {
    await resetText('');
    await type('[in');
    await picker().waitForExist({ timeout: 5_000 });
    await browser.keys('Tab');
    const { value, caret } = await editorState();
    expect(value).toBe(INDEX_LINK);
    expect(caret).toBe(value.length);
  });

  it('inserts on click and keeps focus in the editor', async () => {
    await resetText('A ');
    await type('[');
    const item = $('.wikilink-autocomplete-item');
    await item.waitForExist({ timeout: 5_000 });
    // A synthetic press: WebDriver pointer coordinates land off-target in this
    // frameless window (see tree-editing.e2e.ts).
    await browser.execute(() => {
      document
        .querySelector('.wikilink-autocomplete-item')!
        .dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true, button: 0 }));
    });
    const { value, caret } = await editorState();
    expect(value).toBe(`A ${INDEX_LINK}`);
    expect(caret).toBe(value.length);
    expect(await browser.execute(() => document.activeElement?.tagName)).toBe('TEXTAREA');
  });

  it('moves the selection with ArrowDown/ArrowUp and inserts it on Enter', async () => {
    // With the single fixture object both keys wrap around onto it; the
    // multi-row moves are covered by `moveSelection` unit tests.
    await resetText('B ');
    await type('[');
    await picker().waitForExist({ timeout: 5_000 });
    await browser.keys('ArrowDown');
    await browser.keys('ArrowUp');
    await expect(picker()).toExist();
    await expect($('.wikilink-autocomplete-item.selected .object-title')).toHaveElementProperty('textContent', 'Index');
    // The arrows did not move the editor caret.
    expect((await editorState()).caret).toBe(3);
    await browser.keys('Enter');
    const { value, caret } = await editorState();
    expect(value).toBe(`B ${INDEX_LINK}`);
    expect(caret).toBe(value.length);
  });

  it('shows the empty state for no matches', async () => {
    await resetText('');
    await type('[zzz');
    await picker().waitForExist({ timeout: 5_000 });
    await expect($('.link-picker-empty')).toHaveElementProperty('textContent', 'No objects found');
  });

  it('dismisses with Escape, keeps the text and stays in write mode', async () => {
    await resetText('');
    await type('[In');
    await picker().waitForExist({ timeout: 5_000 });
    await browser.keys('Escape');
    await expect(picker()).not.toExist();
    await type('d');
    await expectClosed();
    expect((await editorState()).value).toBe('[Ind');
    await expect(textarea()).toBeDisplayed();
  });

  it('dismisses with ] and with a space right after [', async () => {
    await resetText('');
    await type('[In');
    await picker().waitForExist({ timeout: 5_000 });
    await type(']');
    await expectClosed();
    expect((await editorState()).value).toBe('[In]');

    await resetText('x ');
    await type('[');
    await picker().waitForExist({ timeout: 5_000 });
    await type(' ');
    await expectClosed();
    expect((await editorState()).value).toBe('x [ ');
  });

  it('does not open for a task-list checkbox or in code', async () => {
    await resetText('- ');
    await type('[');
    await expectClosed();

    await resetText('`code ');
    await type('[');
    await expectClosed();

    await resetText('```\n');
    await type('[');
    await expectClosed();
  });

  it('keeps the [[ trigger', async () => {
    await resetText('');
    await type('[[Ind');
    await picker().waitForExist({ timeout: 5_000 });
    await browser.keys('Enter');
    expect((await editorState()).value).toBe(INDEX_LINK);
  });
});
