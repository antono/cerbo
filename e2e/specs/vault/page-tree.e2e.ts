import { $, browser, expect } from '@wdio/globals';

// Spec `desktop-page-tree`: a page created in a folder nests in the sidebar,
// and moving it to the root through the Move dialog makes the folder vanish.
//
// WebKitWebDriver reports no rendered text for the sidebar rows, so the
// assertions read the DOM (`textContent`) instead of `getText()`.

function folderCount(): Promise<number> {
  return browser.execute(() => document.querySelectorAll('.page-list .item.folder').length);
}

function pageTitles(): Promise<string[]> {
  return browser.execute(() =>
    Array.from(document.querySelectorAll('.page-list .item.page .label'), (el) => el.textContent ?? ''),
  );
}

describe('sidebar page tree', () => {
  it('shows a new page nested under its folder', async () => {
    const index = $('.page-list .page-btn');
    await index.waitForDisplayed({ timeout: 30_000 });

    await browser.keys(['Control', 'n']);
    const title = $('#new-page-title');
    await title.waitForDisplayed({ timeout: 10_000 });
    await title.setValue('Ownership');
    await $('#new-page-path').setValue('notes/rust');
    await browser.keys('Enter');

    await browser.waitUntil(async () => (await folderCount()) === 2, { timeout: 10_000 });
    await expect($('.page-list .item.folder[data-path="notes"] .label')).toHaveElementProperty('textContent', 'notes');
    await expect($('.page-list .item.folder[data-path="notes/rust"] .label')).toHaveElementProperty('textContent', 'rust');
    const nested = $('.page-list .item.page[data-path="notes/rust"]');
    await expect(nested.$('.label')).toHaveElementProperty('textContent', 'Ownership');
    // The new page opened in the editor and is the active row.
    await expect(nested).toHaveElementClass('active');
  });

  it('collapses and expands the folder from its row', async () => {
    const notes = $('.page-list .item.folder[data-path="notes"] .row-btn');
    await notes.click();
    await expect($('.page-list .item.folder[data-path="notes/rust"]')).not.toExist();
    await notes.click();
    await expect($('.page-list .item.folder[data-path="notes/rust"]')).toBeExisting();
  });

  it('collapses with h and expands with l on the focused folder row', async () => {
    const notes = $('.page-list .item.folder[data-path="notes"] .row-btn');
    await notes.click(); // collapse by click…
    await notes.click(); // …and back, leaving focus on the row
    await browser.keys('h');
    await expect($('.page-list .item.folder[data-path="notes/rust"]')).not.toExist();
    await browser.keys('l');
    await expect($('.page-list .item.folder[data-path="notes/rust"]')).toBeExisting();
    // l on an expanded folder steps into it: the focused row is now "rust".
    await browser.keys('l');
    const focusedPath = await browser.execute(() => document.activeElement?.closest('.item')?.getAttribute('data-path'));
    expect(focusedPath).toBe('notes/rust');
  });

  it('renames a folder from its context menu', async () => {
    const rust = $('.page-list .item.folder[data-path="notes/rust"]');
    await rust.click({ button: 'right' });
    const item = $('.row-menu').$('button*=Rename folder');
    await item.waitForDisplayed({ timeout: 5_000 });
    await item.click();

    const name = $('#rename-folder');
    await name.waitForDisplayed({ timeout: 10_000 });
    await expect(name).toHaveValue('rust');
    await browser.keys('Backspace');
    await name.addValue('rustlang');
    await browser.keys('Enter');

    await browser.waitUntil(
      async () => !!(await browser.execute(() => document.querySelector('.page-list .item.folder[data-path="notes/rustlang"]'))),
      { timeout: 10_000 },
    );
    await expect($('.page-list .item.page[data-path="notes/rustlang"] .label')).toHaveElementProperty('textContent', 'Ownership');
    await expect($('.page-list .item.folder[data-path="notes/rust"]')).not.toExist();
  });

  it('moves the page to the root through the Move dialog and drops the folder', async () => {
    const row = $('.page-list .item.page[data-path="notes/rustlang"]');
    await row.moveTo();
    await row.$('button[title="Move to…"]').click();

    const path = $('#move-path');
    await path.waitForDisplayed({ timeout: 10_000 });
    await expect(path).toHaveValue('notes/rustlang');
    // The dialog selects the whole path on open; Backspace clears it through
    // a real input event (clearValue() would not update the bound state).
    await browser.keys('Backspace');
    await expect(path).toHaveValue('');
    await browser.keys('Enter');

    await browser.waitUntil(async () => (await folderCount()) === 0, { timeout: 10_000 });
    expect(await pageTitles()).toEqual(['Index', 'Ownership']);
  });
});
