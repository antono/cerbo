import { $, $$, browser, expect } from '@wdio/globals';

// Spec `desktop-window-toolbar` (change `unify-window-toolbar`, tasks 2.3,
// 2.4, 2.6, 2.7): the page menu, folder segments, the save dot, the ⋯ menu
// and accessible names of every toolbar control.
//
// WebKitWebDriver reports no rendered text for these elements, so the
// assertions read the DOM (`textContent`, attributes) instead of `getText()`.

const FOLDER_PAGE = 'Toolbar Leaf';
const FOLDER = 'Projects/Toolbar';

const tb = (id: string) => $(`[data-testid="${id}"]`);
const menuItem = (action: string) => $(`.row-menu .menu-item[data-action="${action}"]`);

async function openPageRow(title: string) {
  await browser.waitUntil(
    async () => {
      const clicked = await browser.execute((t) => {
        const li = Array.from(document.querySelectorAll('.page-list .item.page')).find(
          (row) => row.querySelector('.label')?.textContent === t,
        );
        (li?.querySelector('.row-btn') as HTMLButtonElement | null)?.click();
        return !!li;
      }, title);
      if (!clicked) return false;
      return (await tb('toolbar-page-menu').getProperty('textContent'))?.toString().includes(title) ?? false;
    },
    { timeout: 10_000, timeoutMsg: `page "${title}" did not open` },
  );
}

describe('window toolbar', () => {
  before(async () => {
    await $('.page-list .page-btn').waitForDisplayed({ timeout: 30_000 });
    // A page inside a nested folder, for the path segments.
    await browser.keys(['Control', 'n']);
    const title = $('#new-page-title');
    await title.waitForDisplayed({ timeout: 10_000 });
    await title.setValue(FOLDER_PAGE);
    await $('#new-page-path').setValue(FOLDER);
    await browser.keys('Enter');
    await title.waitForDisplayed({ reverse: true, timeout: 10_000 });
    await openPageRow(FOLDER_PAGE);
  });

  it('gives every control an accessible name', async () => {
    const unnamed = await browser.execute(() =>
      Array.from(document.querySelectorAll('[data-testid="window-toolbar"] button'))
        .filter((b) => !(b.getAttribute('aria-label') ?? b.textContent ?? '').trim())
        .map((b) => b.outerHTML.slice(0, 120)),
    );
    expect(unnamed).toEqual([]);
    await expect(tb('toolbar-close')).toHaveAttribute('aria-label', 'Quit Cerbo');
    expect(await tb('toolbar-new-page').getAttribute('title')).toMatch(/^New page \((Ctrl|⌘)\+N\)$/);
  });

  it('shows the path with muted folder segments', async () => {
    const segments = await $$('[data-testid="toolbar-segment"]').map((s) => s.getAttribute('data-path'));
    expect(segments).toEqual(['Projects', 'Projects/Toolbar']);
  });

  it('reveals a folder from its segment', async () => {
    // Collapse the parent first, so the reveal has to expand it.
    if (await $('.page-list .item.folder[data-path="Projects/Toolbar"]').isExisting()) {
      await browser.execute(() =>
        (document.querySelector('.page-list .item.folder[data-path="Projects"] .row-btn') as HTMLButtonElement)?.click(),
      );
    }
    await $('.page-list .item.folder[data-path="Projects/Toolbar"]').waitForExist({ reverse: true, timeout: 5_000 });

    await $('[data-testid="toolbar-segment"][data-path="Projects/Toolbar"]').click();
    const row = $('.page-list .item.folder[data-path="Projects/Toolbar"]');
    await row.waitForExist({ timeout: 5_000 });
    await expect(row).toHaveAttribute('aria-selected', 'true');
    await browser.waitUntil(
      () => browser.execute(() => document.activeElement?.closest('.item')?.getAttribute('data-path') === 'Projects/Toolbar'),
      { timeout: 5_000, timeoutMsg: 'the revealed folder row has no focus' },
    );
  });

  it('renames the page from the page menu', async () => {
    await tb('toolbar-page-menu').click();
    const items = await $$('.row-menu .menu-item').map((i) => i.getAttribute('data-action'));
    expect(items).toEqual(['page.rename', 'page.move', 'page.copyLink', 'page.reveal', 'page.delete']);
    await menuItem('page.rename').click();
    const input = $('#rename-title');
    await input.waitForDisplayed({ timeout: 10_000 });
    await expect(input).toHaveValue(FOLDER_PAGE);
    await browser.keys('Escape');
    await input.waitForExist({ reverse: true, timeout: 10_000 });
  });

  it('shows the save dot while an edit is unsaved', async () => {
    await browser.execute(() => (document.activeElement as HTMLElement | null)?.blur());
    if ((await tb('toolbar-mode').getAttribute('data-mode')) !== 'write') await tb('toolbar-mode').click();
    const textarea = $('.carta-input textarea');
    await textarea.waitForDisplayed({ timeout: 10_000 });
    await textarea.click();
    await browser.keys(['End', ' ', '1', '3', '7']);
    await tb('toolbar-save-dot').waitForExist({ timeout: 5_000 });
    await tb('toolbar-save-dot').waitForExist({ reverse: true, timeout: 10_000 });
    expect(await $('.save-indicator').isExisting()).toBe(false);
    await tb('toolbar-mode').click();
    await $('.carta-renderer').waitForExist({ timeout: 10_000 });
  });

  it('toggles both panels and shows them pressed', async () => {
    const left = tb('toolbar-left-panel');
    await expect(left).toHaveAttribute('aria-pressed', 'true');
    await left.click();
    await $('.page-list').waitForExist({ reverse: true, timeout: 5_000 });
    await expect(left).toHaveAttribute('aria-pressed', 'false');
    await left.click();
    await $('.page-list').waitForExist({ timeout: 5_000 });

    const right = tb('toolbar-right-panel');
    const shown = (await right.getAttribute('aria-pressed')) === 'true';
    await right.click();
    await $('.right-panels').waitForExist({ reverse: shown, timeout: 5_000 });
    await right.click();
    await $('.right-panels').waitForExist({ reverse: !shown, timeout: 5_000 });
  });

  it('names the target theme in the ⋯ menu and flips it', async () => {
    const dark = await browser.execute(() => document.documentElement.classList.contains('dark'));
    await tb('toolbar-more').click();
    const theme = menuItem('theme.toggle');
    await theme.waitForDisplayed({ timeout: 5_000 });
    await expect(theme.$('.menu-label')).toHaveElementProperty('textContent', dark ? 'Light theme' : 'Dark theme');
    await theme.click();
    await theme.waitForExist({ reverse: true, timeout: 5_000 });

    await tb('toolbar-more').click();
    await menuItem('theme.toggle').waitForDisplayed({ timeout: 5_000 });
    await expect(menuItem('theme.toggle').$('.menu-label')).toHaveElementProperty(
      'textContent',
      dark ? 'Dark theme' : 'Light theme',
    );
    // Put the theme back.
    await menuItem('theme.toggle').click();
  });

  it('asks before quitting from the ⋯ menu and from the close button', async () => {
    await tb('toolbar-more').click();
    await menuItem('app.quit').waitForDisplayed({ timeout: 5_000 });
    await menuItem('app.quit').click();
    const dialog = $('.confirm-modal');
    await dialog.waitForDisplayed({ timeout: 5_000 });
    await dialog.$('.btn-ghost').click();
    await dialog.waitForExist({ reverse: true, timeout: 5_000 });

    await tb('toolbar-close').click();
    await dialog.waitForDisplayed({ timeout: 5_000 });
    await dialog.$('.btn-ghost').click();
    await dialog.waitForExist({ reverse: true, timeout: 5_000 });
  });
});
