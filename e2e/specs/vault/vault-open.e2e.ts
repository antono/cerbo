import { $, expect } from '@wdio/globals';
import { FIXTURE_VAULT_NAME } from '../../session.ts';

describe('launch with the fixture vault', () => {
  it('opens the seeded vault', async () => {
    // WebKitWebDriver reports no rendered text for the sidebar header, so
    // check the DOM text instead.
    const label = $('.vault-label');
    await label.waitForExist({ timeout: 30_000 });
    await expect(label).toHaveElementProperty('textContent', FIXTURE_VAULT_NAME);
  });

  it('lists the vault pages', async () => {
    // test-vault/ holds a single page object, "Index". Its row is a tree
    // row whose title sits in `.label`; read the DOM text as above.
    const page = $('.page-list .page-btn');
    await page.waitForDisplayed({ timeout: 30_000 });
    await expect(page.$('.label')).toHaveElementProperty('textContent', 'Index');
  });
});
