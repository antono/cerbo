import { $, browser, expect } from '@wdio/globals';

// Spec `command-palette`: `Ctrl+Shift+P` opens page search in command mode,
// and Enter closes search before running the action, so the New Folder dialog
// it opens keeps focus. A second `Ctrl+Shift+P` closes search.

const searchInput = () => $('.search-modal .search-input');
// `$()` is strict and throws when the list holds more than one command.
const commandCount = () => browser.execute(() => document.querySelectorAll('.search-modal .command-item').length);

describe('command mode', () => {
  before(async () => {
    // Wait for the fixture vault so "New folder" (needs a vault) is listed.
    await $('.page-list .page-btn').waitForDisplayed({ timeout: 30_000 });
  });

  it('opens with > typed and closes on a second Ctrl+Shift+P', async () => {
    await browser.keys(['Control', 'Shift', 'p']);
    await searchInput().waitForDisplayed({ timeout: 10_000 });
    await expect(searchInput()).toHaveValue('>');
    await browser.waitUntil(async () => (await commandCount()) > 0, { timeout: 10_000, timeoutMsg: 'no commands listed' });

    await browser.keys(['Control', 'Shift', 'p']);
    await $('.search-modal').waitForExist({ reverse: true, timeout: 10_000 });
  });

  it('switches the toolbar search field to commands with > and back to pages without it', async () => {
    await $('[data-testid="toolbar-search"]').click();
    await searchInput().waitForDisplayed({ timeout: 10_000 });
    await browser.waitUntil(() => browser.execute(() => document.activeElement?.classList.contains('search-input')), {
      timeout: 10_000,
      timeoutMsg: 'the search input does not have focus',
    });

    await browser.keys('>sparql'.split(''));
    const sparql = $('.search-modal .command-item[data-action="sparql.console"]');
    await sparql.waitForDisplayed({ timeout: 10_000 });
    await expect(sparql.$('.result-hotkey')).toHaveText('Ctrl+Shift+E');

    // Delete the `>`: the list shows pages again.
    await browser.keys('Home');
    await browser.keys('Delete');
    await expect(searchInput()).toHaveValue('sparql');
    await browser.waitUntil(async () => (await commandCount()) === 0, { timeout: 10_000, timeoutMsg: 'commands still listed' });

    await browser.keys('Escape');
    await $('.search-modal').waitForExist({ reverse: true, timeout: 10_000 });
  });

  it('runs New folder from >folder + Enter with focus in the dialog', async () => {
    await browser.keys(['Control', 'Shift', 'p']);
    await searchInput().waitForDisplayed({ timeout: 10_000 });
    await browser.keys('folder'.split(''));
    await browser.waitUntil(
      () => browser.execute(() => document.querySelector('.search-modal .command-item.selected')?.getAttribute('data-action')),
      { timeout: 10_000, timeoutMsg: 'no command selected' },
    ).then((id) => expect(id).toBe('folder.new'));
    await browser.keys('Enter');

    await $('.search-modal').waitForExist({ reverse: true, timeout: 10_000 });
    const input = $('#new-folder-path');
    await input.waitForDisplayed({ timeout: 10_000 });
    await browser.waitUntil(() => browser.execute(() => document.activeElement?.id === 'new-folder-path'), {
      timeout: 10_000,
      timeoutMsg: 'the New Folder path field does not have focus',
    });

    await browser.keys('Escape');
    await input.waitForExist({ reverse: true, timeout: 10_000 });
  });
});
