import { $, $$, browser, expect } from '@wdio/globals';

// Spec `desktop-responsive-layout` (change `unify-window-toolbar`, tasks
// 4.2–4.4): the medium floating panel and collapsed path, the narrow drawer,
// sheet and toolbar, and the overlay rules on a 390px window.
//
// The session opens at 1200x760 (e2e/session.ts); each block resizes the
// window and waits for the toolbar's `data-width-class` to follow.
// WebKitWebDriver reports no rendered text for these elements, so the
// assertions read the DOM instead of `getText()`.

const PAGE_A = 'Layout Inbox';
const PAGE_B = 'Layout Leaf';
const FOLDER = 'Projects/Layout';

const tb = (id: string) => $(`[data-testid="${id}"]`);
const byTestId = (id: string) => $(`[data-testid="${id}"]`);

async function resizeTo(width: number, height: number, widthClass: 'wide' | 'medium' | 'narrow') {
  await browser.setWindowSize(width, height);
  await browser.waitUntil(async () => (await tb('window-toolbar').getAttribute('data-width-class')) === widthClass, {
    timeout: 10_000,
    timeoutMsg: `layout did not switch to ${widthClass} at ${width}px`,
  });
}

async function createPage(title: string, path = '') {
  await browser.keys(['Control', 'n']);
  const input = $('#new-page-title');
  await input.waitForDisplayed({ timeout: 10_000 });
  await input.setValue(title);
  if (path) await $('#new-page-path').setValue(path);
  await browser.keys('Enter');
  await input.waitForDisplayed({ reverse: true, timeout: 10_000 });
}

/** Click the tree row of page `title`, in the docked tree or the drawer. */
async function clickPageRow(title: string) {
  await browser.waitUntil(
    () =>
      browser.execute((t) => {
        const li = Array.from(document.querySelectorAll('.page-list .item.page')).find(
          (row) => row.querySelector('.label')?.textContent === t,
        );
        (li?.querySelector('.row-btn') as HTMLButtonElement | null)?.click();
        return !!li;
      }, title),
    { timeout: 10_000, timeoutMsg: `no tree row for "${title}"` },
  );
}

async function openedTitle(): Promise<string> {
  return ((await tb('toolbar-page-menu').getProperty('textContent')) ?? '').toString();
}

const width = (selector: string) =>
  browser.execute((s) => document.querySelector(s)?.getBoundingClientRect().width ?? -1, selector);

describe('responsive layout', () => {
  before(async () => {
    await $('.page-list .page-btn').waitForDisplayed({ timeout: 30_000 });
    await createPage(PAGE_A);
    await createPage(PAGE_B, FOLDER);
    await clickPageRow(PAGE_B);
    await browser.waitUntil(async () => (await openedTitle()).includes(PAGE_B), { timeout: 10_000 });
    // Start every block with the right panel docked in the wide layout.
    if ((await tb('toolbar-right-panel').getAttribute('aria-pressed')) !== 'true') {
      await tb('toolbar-right-panel').click();
    }
    await $('.right-panels').waitForExist({ timeout: 5_000 });
  });

  after(async () => {
    await browser.setWindowSize(1200, 760);
  });

  describe('medium (800px)', () => {
    before(async () => {
      await resizeTo(800, 600, 'medium');
    });

    it('docks no right panel and collapses search and the path', async () => {
      expect(await $('.right-panels').isExisting()).toBe(false);
      await expect(tb('toolbar-right-panel')).toHaveAttribute('aria-pressed', 'false');
      expect(await $('.search-field').isExisting()).toBe(false);
      expect(await $$('[data-testid="toolbar-segment"]').length).toBe(0);
      await expect(tb('toolbar-path-collapsed')).toHaveElementProperty('textContent', '…');
    });

    it('lists the folder segments from "…"', async () => {
      await tb('toolbar-path-collapsed').click();
      const items = await $$('.row-menu .menu-item').map((i) => i.getAttribute('data-action'));
      expect(items).toEqual(['segment:Projects', 'segment:Projects/Layout']);
      await browser.keys('Escape');
      await $('.row-menu').waitForExist({ reverse: true, timeout: 5_000 });
    });

    it('floats the right panel over the editor without changing its width', async () => {
      const before = await width('.editor-wrap');
      await tb('toolbar-right-panel').click();
      await byTestId('right-panel-overlay').waitForDisplayed({ timeout: 5_000 });
      await expect(tb('toolbar-right-panel')).toHaveAttribute('aria-pressed', 'true');
      expect(await width('.editor-wrap')).toBe(before);
      // The toggle closes it again.
      await tb('toolbar-right-panel').click();
      await byTestId('right-panel-overlay').waitForExist({ reverse: true, timeout: 5_000 });
    });
  });

  describe('narrow (390px)', () => {
    before(async () => {
      await resizeTo(390, 844, 'narrow');
    });

    it('shows the page across the full width with a reduced toolbar', async () => {
      expect(await $('.sidebar').isExisting()).toBe(false);
      expect(await $('.right-panels').isExisting()).toBe(false);
      const windowWidth = await browser.execute(() => document.body.clientWidth);
      expect(await width('.editor-wrap')).toBe(windowWidth);
      for (const gone of ['toolbar-vault', 'toolbar-forward', 'toolbar-new-page', 'toolbar-mode', 'toolbar-right-panel']) {
        expect(await tb(gone).isExisting()).toBe(false);
      }
      for (const kept of ['toolbar-left-panel', 'toolbar-back', 'toolbar-page-menu', 'toolbar-search', 'toolbar-more', 'toolbar-close']) {
        await expect(tb(kept)).toBeDisplayed();
      }
      await expect(byTestId('floating-mode')).toBeDisplayed();
    });

    it('closes the drawer when a page is picked in it', async () => {
      await tb('toolbar-left-panel').click();
      const drawer = byTestId('tree-drawer');
      await drawer.waitForDisplayed({ timeout: 5_000 });
      await expect(byTestId('drawer-vault')).toBeDisplayed();
      await expect(byTestId('drawer-new-page')).toBeDisplayed();
      await clickPageRow(PAGE_A);
      await drawer.waitForExist({ reverse: true, timeout: 5_000 });
      await browser.waitUntil(async () => (await openedTitle()).includes(PAGE_A), {
        timeout: 10_000,
        timeoutMsg: `"${PAGE_A}" did not open from the drawer`,
      });
    });

    it('closes an overlay with Esc, its toggle and an outside click', async () => {
      const drawer = byTestId('tree-drawer');

      await tb('toolbar-left-panel').click();
      await drawer.waitForDisplayed({ timeout: 5_000 });
      await browser.keys('Escape');
      await drawer.waitForExist({ reverse: true, timeout: 5_000 });

      await tb('toolbar-left-panel').click();
      await drawer.waitForDisplayed({ timeout: 5_000 });
      await tb('toolbar-left-panel').click();
      await drawer.waitForExist({ reverse: true, timeout: 5_000 });

      // Outside the drawer: the backdrop's right part, over the editor in
      // write mode. The click SHALL NOT edit the page: the backdrop takes the
      // pointer, the textarea gets no focus and its text and caret stay.
      const mode = await byTestId('floating-mode').getAttribute('data-mode');
      if (mode !== 'write') await byTestId('floating-mode').click();
      const textarea = $('.carta-input textarea');
      await textarea.waitForDisplayed({ timeout: 10_000 });
      await browser.execute(() => (document.activeElement as HTMLElement | null)?.blur());
      const editorState = () =>
        browser.execute(() => {
          const ta = document.querySelector('.carta-input textarea') as HTMLTextAreaElement | null;
          return {
            value: ta?.value ?? null,
            start: ta?.selectionStart ?? null,
            end: ta?.selectionEnd ?? null,
            focused: !!ta && document.activeElement === ta,
          };
        });
      const before = await editorState();
      await tb('toolbar-left-panel').click();
      await drawer.waitForDisplayed({ timeout: 5_000 });
      await browser.execute(() => {
        const w = window as unknown as { __downTarget: string | null };
        w.__downTarget = null;
        window.addEventListener(
          'pointerdown',
          (e) => { w.__downTarget = (e.target as HTMLElement | null)?.dataset?.testid ?? ''; },
          { capture: true, once: true },
        );
      });
      await byTestId('overlay-backdrop').click({ x: 150 });
      await drawer.waitForExist({ reverse: true, timeout: 5_000 });
      expect(await browser.execute(() => (window as unknown as { __downTarget: string | null }).__downTarget)).toBe(
        'overlay-backdrop',
      );
      const after = await editorState();
      expect(after.focused).toBe(false);
      expect(after).toEqual({ ...before, focused: false });
      await expect(byTestId('floating-mode')).toHaveAttribute('data-mode', 'write');
      if (mode !== 'write') await byTestId('floating-mode').click();
    });

    it('asks before deleting from the page menu, with no tree mounted', async () => {
      expect(await $('.page-list').isExisting()).toBe(false);
      await tb('toolbar-page-menu').click();
      const entry = $('.row-menu .menu-item[data-action="page.delete"]');
      await entry.waitForDisplayed({ timeout: 5_000 });
      // Menu targets are at least 32px tall in the narrow layout.
      const heights = await browser.execute(() =>
        Array.from(document.querySelectorAll('.row-menu .menu-item')).map((i) => i.getBoundingClientRect().height),
      );
      expect(Math.min(...heights)).toBeGreaterThanOrEqual(32);
      await entry.click();
      const dialog = $('.confirm-modal');
      await dialog.waitForDisplayed({ timeout: 5_000 });
      // Cancel: the page stays and hotkeys work again.
      await browser.keys('Escape');
      await dialog.waitForExist({ reverse: true, timeout: 5_000 });
      expect(await openedTitle()).toContain(PAGE_A);
    });

    it('opens the right panel from ⋯ as a sheet with tabs, closing the drawer', async () => {
      await tb('toolbar-left-panel').click();
      await byTestId('tree-drawer').waitForDisplayed({ timeout: 5_000 });

      await tb('toolbar-more').click();
      const entry = $('.row-menu .menu-item[data-action="panel.right"]');
      await entry.waitForDisplayed({ timeout: 5_000 });
      await entry.click();

      const sheet = byTestId('right-panel-sheet');
      await sheet.waitForDisplayed({ timeout: 5_000 });
      // The drawer leaves with a 160ms fly-out transition.
      await byTestId('tree-drawer').waitForExist({ reverse: true, timeout: 5_000 });
      const tabs = await $$('[data-testid^="sheet-tab-"]').map((t) => t.getAttribute('data-testid'));
      expect(tabs).toEqual(['sheet-tab-backlinks', 'sheet-tab-similar', 'sheet-tab-files']);
      await byTestId('sheet-tab-files').click();
      await expect(sheet.$('.attachments-panel')).toBeExisting();

      await browser.keys('Escape');
      await sheet.waitForExist({ reverse: true, timeout: 5_000 });
    });
  });

  describe('back to wide', () => {
    it('docks the remembered panels again', async () => {
      await resizeTo(1200, 760, 'wide');
      await $('.right-panels').waitForExist({ timeout: 5_000 });
      await $('.sidebar').waitForExist({ timeout: 5_000 });
      await expect(tb('toolbar-right-panel')).toHaveAttribute('aria-pressed', 'true');
      await expect(tb('toolbar-left-panel')).toHaveAttribute('aria-pressed', 'true');
      expect(await byTestId('tree-drawer').isExisting()).toBe(false);
    });
  });
});
