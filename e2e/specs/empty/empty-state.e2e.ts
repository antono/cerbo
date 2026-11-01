import { $, expect } from '@wdio/globals';

describe('first launch without vaults', () => {
  it('shows the add-vault prompt', async () => {
    const prompt = $('.no-vault p');
    await prompt.waitForDisplayed({ timeout: 30_000 });
    await expect(prompt).toHaveText('Add a vault to get started.');
  });
});
