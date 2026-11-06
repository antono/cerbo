import { $, browser, expect } from '@wdio/globals';

// Specs `desktop-tree-editing` and `sidebar-navigation` (preview mode):
// reorder and move rows by pointer drag, keyboard and menu, folder drag, the
// Add menu, and empty folders. The drags are real pointer actions, which the
// pointer-event implementation receives like a user's mouse.
//
// WebKitWebDriver reports no rendered text for the sidebar rows, so the
// assertions read the DOM (`textContent`) instead of `getText()`.

/** Visible rows in order: folders as `path/`, pages as `path/Title` (or `Title` at the root). */
function rows(): Promise<string[]> {
  return browser.execute(() =>
    Array.from(document.querySelectorAll('.page-list .item'), (li) => {
      const path = li.getAttribute('data-path') ?? '';
      const label = li.querySelector('.label')?.textContent ?? '';
      return li.getAttribute('data-kind') === 'folder' ? `${path}/` : path ? `${path}/${label}` : label;
    }),
  );
}

async function expectRows(expected: string[]) {
  await browser.waitUntil(async () => JSON.stringify(await rows()) === JSON.stringify(expected), {
    timeout: 10_000,
    timeoutMsg: `rows were ${JSON.stringify(await rows())}, expected ${JSON.stringify(expected)}`,
  });
}

async function pageRow(title: string) {
  const uuid = await browser.execute(
    (t) =>
      Array.from(document.querySelectorAll('.page-list .item.page'))
        .find((li) => li.querySelector('.label')?.textContent === t)
        ?.getAttribute('data-uuid'),
    title,
  );
  if (!uuid) throw new Error(`no page row "${title}"`);
  return $(`.page-list .item.page[data-uuid="${uuid}"]`);
}

function folderRow(path: string) {
  return $(`.page-list .item.folder[data-path="${path}"]`);
}

function focusedLabel(): Promise<string | null> {
  return browser.execute(() => document.activeElement?.closest('.item')?.querySelector('.label')?.textContent ?? null);
}

async function createPage(title: string, path = '') {
  await browser.keys(['Control', 'n']);
  const input = $('#new-page-title');
  await input.waitForDisplayed({ timeout: 10_000 });
  await input.setValue(title);
  if (path) await $('#new-page-path').setValue(path);
  await browser.keys('Enter');
  await browser.waitUntil(async () => (await rows()).some((r) => r.endsWith(title)), { timeout: 10_000 });
  // The next Ctrl+N must not race the closing dialog.
  await input.waitForDisplayed({ reverse: true, timeout: 10_000 });
}

type Row = Awaited<ReturnType<typeof folderRow>>;

// WebKitWebDriver maps action coordinates onto this frameless window with a
// small scale (about 0.96), so a point aimed by client coordinates lands a few
// pixels off. `calibrate` measures the factor once; `pointIn` corrects for it.
let scale = { x: 1, y: 1 };

async function calibrate() {
  await browser.execute(() => {
    window.addEventListener('pointermove', (e) => ((window as any).__lastPointer = [e.clientX, e.clientY]), true);
  });
  await browser.action('pointer').move({ origin: 'viewport', x: 200, y: 400 }).perform();
  const [cx, cy] = await browser.execute(() => (window as any).__lastPointer as [number, number]);
  scale = { x: 200 / cx, y: 400 / cy };
}

/** A viewport point inside `row`, `fraction` of its height from the top. */
async function pointIn(row: Row, fraction: number) {
  const el = await row;
  await el.waitForExist({ timeout: 10_000 });
  const r = await browser.execute((e: HTMLElement) => e.getBoundingClientRect().toJSON(), el as unknown as HTMLElement);
  return { x: Math.round((r.left + 24) * scale.x), y: Math.round((r.top + r.height * fraction) * scale.y) };
}

/**
 * Click the centre of `el` through the calibrated pointer (WebDriver's own
 * element click lands a row too high near the bottom of the window).
 */
async function clickOn(el: Row, button: 0 | 2 = 0) {
  const p = await pointIn(el, 0.5);
  await browser.action('pointer').move({ origin: 'viewport', ...p }).down({ button }).up({ button }).perform();
}

/** The entry `label` of the open row menu. */
function menuItem(label: string) {
  return $('.row-menu').$(`button*=${label}`);
}

/** Press on `src`, move past the drag threshold, then to `fraction` of `dst`'s height, and release. */
async function drag(src: Row, dst: Row, fraction: number) {
  const from = await pointIn(src, 0.5);
  const to = await pointIn(dst, fraction);
  await browser
    .action('pointer')
    .move({ origin: 'viewport', ...from })
    .down({ button: 0 })
    .move({ origin: 'viewport', x: from.x, y: from.y + 10 })
    .move({ origin: 'viewport', ...to })
    .pause(100)
    .up({ button: 0 })
    .perform();
}

// Drop zones (design D7) as fractions of the target row's height.
const TOP_QUARTER = 0.12;
const BOTTOM_HALF = 0.75;
const MIDDLE = 0.5;

describe('sidebar tree editing', () => {
  it('starts from two root pages and a page in a folder', async () => {
    await $('.page-list .page-btn').waitForDisplayed({ timeout: 30_000 });
    await createPage('Alpha');
    await createPage('Beta');
    await createPage('Own', 'notes');
    await expectRows(['notes/', 'notes/Own', 'Alpha', 'Beta', 'Index']);
    await calibrate();
  });

  it('moves focus with j and ArrowDown in preview mode', async () => {
    await browser.keys('Escape'); // leave the editor: preview mode
    await expect($('.page-editor.write-mode')).not.toExist();
    await (await pageRow('Alpha')).$('.row-btn').click();
    await browser.keys('j');
    expect(await focusedLabel()).toBe('Beta');
    await browser.keys('ArrowDown');
    expect(await focusedLabel()).toBe('Index');
    await expect($('.page-editor.write-mode')).not.toExist();
  });

  it('moves the focused row up with Alt+ArrowUp and keeps focus on it', async () => {
    await browser.keys(['Alt', 'ArrowUp']);
    await expectRows(['notes/', 'notes/Own', 'Alpha', 'Index', 'Beta']);
    expect(await focusedLabel()).toBe('Index');
  });

  it('drags a page before another row', async () => {
    await drag(await pageRow('Beta'), await pageRow('Alpha'), TOP_QUARTER);
    await expectRows(['notes/', 'notes/Own', 'Beta', 'Alpha', 'Index']);
  });

  it('drags a page into a folder, right after a row there', async () => {
    await drag(await pageRow('Alpha'), await pageRow('Own'), BOTTOM_HALF);
    await expectRows(['notes/', 'notes/Own', 'notes/Alpha', 'Beta', 'Index']);
  });

  it('adds an empty folder from the Add menu on empty sidebar space', async () => {
    await clickOn($('.page-list .root-drop'), 2);
    const item = menuItem('New folder');
    await item.waitForDisplayed({ timeout: 5_000 });
    await clickOn(item);
    const input = $('#new-folder-path');
    await input.waitForDisplayed({ timeout: 10_000 });
    await input.setValue('archive');
    await browser.keys('Enter');
    await expectRows(['notes/', 'notes/Own', 'notes/Alpha', 'Beta', 'Index', 'archive/']);
  });

  it('drags a folder into another folder with its pages', async () => {
    await drag(folderRow('notes'), folderRow('archive'), MIDDLE);
    await expectRows(['Beta', 'Index', 'archive/', 'archive/notes/', 'archive/notes/Own', 'archive/notes/Alpha']);
  });

  it('shows no drop indicator over a folder’s own subtree, and Escape cancels the drag', async () => {
    const before = await rows();
    const from = await pointIn(folderRow('archive'), 0.5);
    const to = await pointIn(folderRow('archive/notes'), MIDDLE);
    await browser
      .action('pointer')
      .move({ origin: 'viewport', ...from })
      .down({ button: 0 })
      .move({ origin: 'viewport', x: from.x, y: from.y + 10 })
      .move({ origin: 'viewport', ...to })
      .perform(true);
    await expect($('.drag-ghost')).toBeExisting();
    expect(await browser.execute(() => document.querySelectorAll('.drop-before, .drop-after, .drop-inside').length)).toBe(0);
    await browser.keys('Escape');
    await expect($('.drag-ghost')).not.toExist();
    await browser.releaseActions();
    expect(await rows()).toEqual(before);
  });

  it('moves a row down from its menu', async () => {
    await clickOn(await pageRow('Beta'), 2);
    const item = menuItem('Move down');
    await item.waitForDisplayed({ timeout: 5_000 });
    await clickOn(item);
    await expectRows(['Index', 'Beta', 'archive/', 'archive/notes/', 'archive/notes/Own', 'archive/notes/Alpha']);
  });

  it('offers Delete folder only for a folder without pages', async () => {
    await clickOn(folderRow('archive'), 2);
    await menuItem('Rename folder').waitForDisplayed({ timeout: 5_000 });
    await expect(menuItem('Delete folder')).not.toExist();
    await browser.keys('Escape');

    await $('.page-list button[title="New folder"]').click();
    const input = $('#new-folder-path');
    await input.waitForDisplayed({ timeout: 10_000 });
    await input.setValue('inbox');
    await browser.keys('Enter');
    await folderRow('inbox').waitForExist({ timeout: 10_000 });

    await clickOn(folderRow('inbox'), 2);
    const remove = menuItem('Delete folder');
    await remove.waitForDisplayed({ timeout: 5_000 });
    await clickOn(remove);
    await folderRow('inbox').waitForExist({ reverse: true, timeout: 10_000 });
  });

  it('refuses a folder that already exists in the New Folder dialog', async () => {
    await $('.page-list button[title="New folder"]').click();
    const input = $('#new-folder-path');
    await input.waitForDisplayed({ timeout: 10_000 });
    await input.setValue('archive');
    // Enter would pick the open suggestion (archive/notes); use the button.
    await clickOn($('.dialog-modal .primary-btn'));
    const error = $('.dialog-modal .error-msg');
    await error.waitForExist({ timeout: 10_000 });
    await expect(error).toHaveElementProperty('textContent', 'folder already exists: archive');
    await browser.keys('Escape');
  });
});
