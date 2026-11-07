import { $, $$, browser, expect } from '@wdio/globals';
import { FIXTURE_VAULT_NAME, LONG_PATH_VAULT_DIR } from '../../session.ts';

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

  it('shows the vault path in the selector', async () => {
    // Spec `vault-selector`: each row shows the absolute path beside the
    // name, with the full path as the row tooltip. session.ts copies the
    // fixture vault to `<tmp>/vault`.
    await $('.vault-name-btn').click();
    const row = $$('.vault-selector-modal .vault-item')[0];
    await row.waitForExist({ timeout: 10_000 });
    await expect(row.$('.vault-item-name')).toHaveElementProperty('textContent', FIXTURE_VAULT_NAME);
    const path = await row.$('.vault-item-path').getProperty('textContent');
    expect(path).toMatch(/^\/.+\/vault$/);
    await expect(row).toHaveAttribute('title', path as string);
    await expect(row.$('.vault-item-path-tail')).toHaveElementProperty('textContent', '/vault');
    await browser.keys('Escape');
  });

  it('tells same-named vaults apart and truncates a long path in the middle', async () => {
    // session.ts registers a second vault named like the fixture vault, at a
    // path far wider than its row.
    await $('.vault-name-btn').click();
    const rows = $$('.vault-selector-modal .vault-item');
    await rows[1].waitForExist({ timeout: 10_000 });
    await expect(rows).toBeElementsArrayOfSize(2);
    await expect(rows[1].$('.vault-item-name')).toHaveElementProperty('textContent', FIXTURE_VAULT_NAME);
    expect(await rows[1].getAttribute('title')).not.toBe(await rows[0].getAttribute('title'));

    // The head is cut with an ellipsis; the folder name stays whole.
    const head = rows[1].$('.vault-item-path-head');
    const tail = rows[1].$('.vault-item-path-tail');
    await expect(tail).toHaveElementProperty('textContent', `/${LONG_PATH_VAULT_DIR}`);
    expect(await head.getProperty('scrollWidth')).toBeGreaterThan((await head.getProperty('clientWidth')) as number);
    expect(await tail.getProperty('scrollWidth')).toBeLessThanOrEqual((await tail.getProperty('clientWidth')) as number);
    expect(await tail.getProperty('clientWidth')).toBeGreaterThan(0);
    expect(await rows[1].getAttribute('title')).toMatch(new RegExp(`/${LONG_PATH_VAULT_DIR}$`));
    await browser.keys('Escape');
  });
});
