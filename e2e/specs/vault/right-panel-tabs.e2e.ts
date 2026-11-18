import { $, $$, browser, expect } from '@wdio/globals';

// Spec `desktop-right-panel` and `desktop-outline-panel` (change
// `add-right-panel-tabs`): the icon tab strip, switching, the remembered tab,
// the tab hotkeys, the Outline, and the same strip in the narrow sheet.
//
// The window starts at 1200x760 (e2e/session.ts), the wide layout, so the
// panel is docked. WebKitWebDriver reports no rendered text for these
// elements, so the assertions read the DOM instead of `getText()`.

const PAGE = 'Tabs Outline Page';
const OTHER = 'Tabs Other Page';
const TAB_IDS = ['attachments', 'links', 'outline', 'agent'];

const byTestId = (id: string) => $(`[data-testid="${id}"]`);
const tab = (id: string) => byTestId(`right-tab-${id}`);

async function createPage(title: string) {
  await browser.keys(['Control', 'n']);
  const input = $('#new-page-title');
  await input.waitForDisplayed({ timeout: 10_000 });
  await input.setValue(title);
  await browser.keys('Enter');
  await input.waitForDisplayed({ reverse: true, timeout: 10_000 });
}

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
  return ((await byTestId('toolbar-page-menu').getProperty('textContent')) ?? '').toString();
}

/** The ids of the tabs marked selected: exactly one is expected. */
function selectedTabs(): Promise<string[]> {
  return browser.execute(() =>
    Array.from(document.querySelectorAll('[data-testid^="right-tab-"][role="tab"][aria-selected="true"]')).map((t) =>
      (t.getAttribute('data-testid') ?? '').replace('right-tab-', ''),
    ),
  );
}

async function expectSelected(id: string) {
  await browser.waitUntil(async () => (await selectedTabs()).join() === id, {
    timeout: 5_000,
    timeoutMsg: `expected only ${id} selected`,
  });
}

/** The ids of the sub-panels on screen: exactly the selected one. */
function shownPanels(): Promise<string[]> {
  return browser.execute(() =>
    Array.from(document.querySelectorAll('[role="tabpanel"][data-testid^="right-panel-"]'))
      .filter((p) => getComputedStyle(p).visibility !== 'hidden')
      .map((p) => (p.getAttribute('data-testid') ?? '').replace('right-panel-', '')),
  );
}

/** Ctrl+Alt+<digit>: the tab hotkeys. */
async function hotkey(digit: string) {
  await browser.keys(['Control', 'Alt', digit]);
}

function outlineTitles(): Promise<string[]> {
  return browser.execute(() =>
    Array.from(document.querySelectorAll('[data-testid="outline-item"]')).map((b) => b.getAttribute('title') ?? ''),
  );
}

describe('right panel tabs', () => {
  before(async () => {
    await $('.page-list .page-btn').waitForDisplayed({ timeout: 30_000 });
    await createPage(PAGE);
    await createPage(OTHER);
    await clickPageRow(PAGE);
    await browser.waitUntil(async () => (await openedTitle()).includes(PAGE), { timeout: 10_000 });
    const showPanels = $('[data-testid="toolbar-right-panel"][aria-pressed="false"]');
    if (await showPanels.isExisting()) await showPanels.click();
    await $('.right-panels').waitForDisplayed({ timeout: 5_000 });
  });

  it('shows the icon strip in order, each tab named and with a hotkey tooltip', async () => {
    const tabs = await $$('[data-testid^="right-tab-"][role="tab"]').map((t) => t.getAttribute('data-testid'));
    expect(tabs).toEqual(TAB_IDS.map((id) => `right-tab-${id}`));
    for (const id of TAB_IDS) {
      const label = await tab(id).getAttribute('aria-label');
      expect(label?.toLowerCase()).toContain(id);
      expect(await tab(id).getAttribute('title')).toMatch(/\(Ctrl\+Alt\+\d\)$/);
    }
    expect(await tab('outline').getAttribute('title')).toBe('Outline (Ctrl+Alt+3)');
  });

  it('starts on Links and shows only the selected sub-panel', async () => {
    await expectSelected('links');
    expect(await shownPanels()).toEqual(['links']);
  });

  it('switches tab on click', async () => {
    await tab('attachments').click();
    await expectSelected('attachments');
    expect(await shownPanels()).toEqual(['attachments']);
    await tab('links').click();
    await expectSelected('links');
    expect(await shownPanels()).toEqual(['links']);
  });

  it('shows Backlinks, Outgoing links and Similar notes in Links', async () => {
    for (const section of ['backlinks-section', 'outgoing-section']) {
      await expect(byTestId(section)).toBeExisting();
    }
    expect(await $('.similar-section').isExisting()).toBe(true);
  });

  it('keeps the tab when another page opens', async () => {
    await tab('outline').click();
    await expectSelected('outline');
    await clickPageRow(OTHER);
    await browser.waitUntil(async () => (await openedTitle()).includes(OTHER), { timeout: 10_000 });
    await expectSelected('outline');
    expect(await shownPanels()).toEqual(['outline']);
  });

  it('remembers the tab across a restart', async () => {
    await tab('attachments').click();
    await expectSelected('attachments');
    // The selection is saved to ui.toml; reloading the webview reads it back.
    await browser.pause(500);
    await browser.refresh();
    await $('.page-list .page-btn').waitForDisplayed({ timeout: 30_000 });
    await $('[data-testid="right-tabs"]').waitForDisplayed({ timeout: 15_000 });
    await expectSelected('attachments');
  });

  it('opens a hidden panel with the hotkey, switches, and hides on the selected tab', async () => {
    // Hide the panel with the toolbar toggle first.
    await byTestId('toolbar-right-panel').click();
    await $('.right-panels').waitForExist({ reverse: true, timeout: 5_000 });

    await hotkey('3');
    await $('.right-panels').waitForDisplayed({ timeout: 5_000 });
    await expectSelected('outline');

    await hotkey('2');
    await expectSelected('links');
    expect(await $('.right-panels').isDisplayed()).toBe(true);

    await hotkey('2');
    await $('.right-panels').waitForExist({ reverse: true, timeout: 5_000 });
  });

  describe('outline', () => {
    before(async () => {
      await clickPageRow(PAGE);
      await browser.waitUntil(async () => (await openedTitle()).includes(PAGE), { timeout: 10_000 });
      await hotkey('3');
      await $('.right-panels').waitForDisplayed({ timeout: 5_000 });
      await expectSelected('outline');
      await browser.execute(() => (document.activeElement as HTMLElement | null)?.blur());
      await browser.keys('i');
      await $('.carta-input textarea').waitForDisplayed({ timeout: 10_000 });
    });

    it('lists the headings while typing and skips code blocks', async () => {
      await browser.execute(() => {
        const t = document.querySelector('.carta-input textarea') as HTMLTextAreaElement;
        t.value = ['# Plan', '', '## Budget', '', '```', '# not a heading', '```', '', '## Risks', ''].join('\n');
        t.dispatchEvent(new Event('input', { bubbles: true }));
      });
      await browser.waitUntil(async () => (await outlineTitles()).join('|') === 'Plan|Budget|Risks', {
        timeout: 5_000,
        timeoutMsg: 'outline did not list Plan, Budget and Risks',
      });
      const levels = await $$('[data-testid="outline-item"]').map((b) => b.getAttribute('data-level'));
      expect(levels).toEqual(['1', '2', '2']);
    });

    it('adds a heading as it is typed', async () => {
      await browser.execute(() => {
        const t = document.querySelector('.carta-input textarea') as HTMLTextAreaElement;
        t.focus();
        t.setSelectionRange(t.value.length, t.value.length);
      });
      await browser.keys(['#', '#', ' ', 'N', 'o', 't', 'e', 's']);
      await browser.waitUntil(async () => (await outlineTitles()).includes('Notes'), { timeout: 5_000 });
    });

    it('jumps to a heading and puts the cursor on it in edit mode', async () => {
      const items = await $$('[data-testid="outline-item"]');
      const risks = (await outlineTitles()).indexOf('Risks');
      await items[risks].click();
      await browser.waitUntil(
        () =>
          browser.execute(() => {
            const t = document.querySelector('.carta-input textarea') as HTMLTextAreaElement;
            return t.value.slice(t.selectionStart).startsWith('## Risks');
          }),
        { timeout: 5_000, timeoutMsg: 'cursor did not move to the Risks heading' },
      );
      await browser.waitUntil(
        async () => (await $('[data-testid="outline-item"][aria-current="true"]').getAttribute('title')) === 'Risks',
        { timeout: 5_000, timeoutMsg: 'Risks was not marked as the current heading' },
      );
    });

    it('shows an empty state for a page without headings', async () => {
      await browser.execute(() => {
        const t = document.querySelector('.carta-input textarea') as HTMLTextAreaElement;
        t.value = 'no headings here\n';
        t.dispatchEvent(new Event('input', { bubbles: true }));
      });
      await $('[data-testid="outline-empty"]').waitForDisplayed({ timeout: 5_000 });
    });
  });

  describe('narrow sheet', () => {
    after(async () => {
      await browser.setWindowSize(1200, 760);
      await browser.waitUntil(
        async () => (await byTestId('window-toolbar').getAttribute('data-width-class')) === 'wide',
        { timeout: 10_000 },
      );
    });

    it('uses the same strip and the same selected tab', async () => {
      await browser.setWindowSize(390, 760);
      await browser.waitUntil(
        async () => (await byTestId('window-toolbar').getAttribute('data-width-class')) === 'narrow',
        { timeout: 10_000, timeoutMsg: 'layout did not switch to narrow' },
      );
      await byTestId('toolbar-more').click();
      const entry = $('.row-menu .menu-item[data-action="panel.right"]');
      await entry.waitForDisplayed({ timeout: 5_000 });
      await entry.click();

      const sheet = byTestId('right-panel-sheet');
      await sheet.waitForDisplayed({ timeout: 5_000 });
      // Outline was selected in the wide layout and stays selected here.
      await expectSelected('outline');
      expect(await sheet.$('[data-testid="right-tab-outline"]').getAttribute('aria-selected')).toBe('true');
      expect(await $$('[data-testid^="sheet-tab-"]').length).toBe(0);
      expect(await shownPanels()).toEqual(['outline']);

      await sheet.$('[data-testid="right-tab-links"]').click();
      await expectSelected('links');
      expect(await shownPanels()).toEqual(['links']);
    });
  });
});
