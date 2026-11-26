import { $, browser, expect } from '@wdio/globals';

// Spec `keyboard-shortcuts`, "Interface Zoom Shortcuts": Ctrl+Shift+Plus,
// Ctrl+Shift+Minus and Ctrl+Shift+0, and the same actions in the ⋯ menu,
// scale the whole webview. A zoomed webview has fewer CSS pixels across the
// same window, so `innerWidth` shows the factor.

const tb = (id: string) => $(`[data-testid="${id}"]`);
const menuItem = (action: string) => $(`.row-menu .menu-item[data-action="${action}"]`);
const cssWidth = () => browser.execute(() => window.innerWidth);

/** Wait until the webview is zoomed to `factor` relative to `base` CSS pixels. */
async function expectZoom(base: number, factor: number) {
  await browser.waitUntil(async () => Math.abs((await cssWidth()) * factor - base) <= 2 * factor, {
    timeout: 5_000,
    timeoutMsg: `the webview is not at ${factor * 100}% (base ${base}px, now ${await cssWidth()}px)`,
  });
}

async function menuLabel(action: string): Promise<string> {
  await tb('toolbar-more').click();
  const item = menuItem(action);
  await item.waitForDisplayed({ timeout: 5_000 });
  const label = String(await item.$('.menu-label').getProperty('textContent'));
  await browser.keys('Escape');
  await item.waitForExist({ reverse: true, timeout: 5_000 });
  return label;
}

describe('interface zoom', () => {
  let base = 0;

  before(async () => {
    await $('.page-list .page-btn').waitForDisplayed({ timeout: 30_000 });
    base = await cssWidth();
  });

  after(async () => {
    await browser.keys(['Control', 'Shift', '0']);
  });

  it('zooms in and out with Ctrl+Shift+Plus and Ctrl+Shift+Minus', async () => {
    await browser.keys(['Control', 'Shift', '=']);
    await expectZoom(base, 1.1);
    await browser.keys(['Control', 'Shift', '=']);
    await expectZoom(base, 1.25);
    expect(await menuLabel('view.zoomIn')).toBe('Zoom in (125%)');

    await browser.keys(['Control', 'Shift', '-']);
    await expectZoom(base, 1.1);
    await browser.keys(['Control', 'Shift', '0']);
    await expectZoom(base, 1);
  });

  it('zooms from the ⋯ menu', async () => {
    await tb('toolbar-more').click();
    await menuItem('view.zoomOut').waitForDisplayed({ timeout: 5_000 });
    await menuItem('view.zoomOut').click();
    await expectZoom(base, 0.9);
    expect(await menuLabel('view.zoomOut')).toBe('Zoom out (90%)');

    await tb('toolbar-more').click();
    await menuItem('view.zoomReset').waitForDisplayed({ timeout: 5_000 });
    await menuItem('view.zoomReset').click();
    await expectZoom(base, 1);
  });
});
