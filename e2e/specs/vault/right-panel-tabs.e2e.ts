import { $, $$, browser, expect } from '@wdio/globals';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

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

/** Start counting the commands the page sends to the backend, by name. */
function countCommands() {
  return browser.execute(() => {
    const internals = (window as unknown as {
      __TAURI_INTERNALS__: { invoke: (c: string, ...rest: unknown[]) => Promise<unknown> };
    }).__TAURI_INTERNALS__;
    const w = window as unknown as { __cmdCounts?: Record<string, number>; __cmdWrapped?: boolean };
    w.__cmdCounts = {};
    if (w.__cmdWrapped) return;
    w.__cmdWrapped = true;
    const original = internals.invoke.bind(internals);
    internals.invoke = (cmd: string, ...rest: unknown[]) => {
      w.__cmdCounts![cmd] = (w.__cmdCounts![cmd] ?? 0) + 1;
      return original(cmd, ...rest);
    };
  });
}

function commandCount(cmd: string): Promise<number> {
  return browser.execute((c) => (window as unknown as { __cmdCounts?: Record<string, number> }).__cmdCounts?.[c] ?? 0, cmd);
}

/** The Outline panel's scroller. */
const OUTLINE_SCROLLER = '[data-testid="outline-panel"] .panel-content';

function setEditorText(text: string) {
  return browser.execute((t) => {
    const ta = document.querySelector('.carta-input textarea') as HTMLTextAreaElement;
    ta.value = t;
    ta.dispatchEvent(new Event('input', { bubbles: true }));
  }, text);
}

/** `## Section 1` ... with enough body lines under each to make the page scroll. */
function longPage(sections: number): string {
  const body = Array.from({ length: 8 }, (_, i) => `Line ${i + 1} of the section.`).join('\n\n');
  return Array.from({ length: sections }, (_, i) => `## Section ${i + 1}\n\n${body}\n`).join('\n');
}

function currentOutlineTitle(): Promise<string | null> {
  return browser.execute(
    () => document.querySelector('[data-testid="outline-item"][aria-current="true"]')?.getAttribute('title') ?? null,
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

  describe('loading', () => {
    it('sends no links or similar notes request while another tab is selected', async () => {
      await hotkey('3');
      await $('.right-panels').waitForDisplayed({ timeout: 5_000 });
      await expectSelected('outline');
      await countCommands();
      await clickPageRow(PAGE);
      await browser.waitUntil(async () => (await openedTitle()).includes(PAGE), { timeout: 10_000 });
      await browser.waitUntil(async () => (await commandCount('page_read')) > 0, { timeout: 10_000 });
      await browser.pause(1_000);
      expect(await commandCount('similar_get')).toBe(0);
      expect(await commandCount('backlinks_get')).toBe(0);
      expect(await commandCount('outgoing_links_get')).toBe(0);
    });

    it('loads backlinks once per page when Links is selected', async () => {
      await countCommands();
      await hotkey('2');
      await expectSelected('links');
      await browser.waitUntil(async () => (await commandCount('backlinks_get')) > 0, { timeout: 10_000 });
      await browser.pause(1_000);
      expect(await commandCount('backlinks_get')).toBe(1);
      expect(await commandCount('outgoing_links_get')).toBe(1);

      // With Links selected, opening a page loads its lists once, not twice.
      await countCommands();
      await clickPageRow(OTHER);
      await browser.waitUntil(async () => (await openedTitle()).includes(OTHER), { timeout: 10_000 });
      await browser.waitUntil(async () => (await commandCount('backlinks_get')) > 0, { timeout: 10_000 });
      await browser.pause(1_000);
      expect(await commandCount('backlinks_get')).toBe(1);
    });
  });

  describe('badges', () => {
    let fixtures: string;

    before(async () => {
      fixtures = mkdtempSync(join(tmpdir(), 'cerbo-tabs-'));
      writeFileSync(join(fixtures, 'tabs-137.pdf'), '%PDF-1.4\n% 137\n%%EOF\n');
      await clickPageRow(PAGE);
      await browser.waitUntil(async () => (await openedTitle()).includes(PAGE), { timeout: 10_000 });
      await browser.execute(() => (document.activeElement as HTMLElement | null)?.blur());
      await browser.keys('i');
      await $('.carta-input textarea').waitForDisplayed({ timeout: 10_000 });
    });

    after(async () => {
      rmSync(fixtures, { recursive: true, force: true });
      // Back to preview, so the next block enters edit mode itself.
      await browser.execute(() => (document.activeElement as HTMLElement | null)?.blur());
      await browser.keys(['Control', 'e']);
      await $('.carta-input textarea').waitForDisplayed({ reverse: true, timeout: 10_000 });
    });

    it('has no count badge on a page without attachments', async () => {
      expect(await byTestId('right-tab-badge-attachments').isExisting()).toBe(false);
      expect(await tab('attachments').getAttribute('aria-label')).toBe('Attachments');
    });

    it('shows the attachment count on the tab while Links is selected', async () => {
      await tab('links').click();
      await expectSelected('links');
      await browser.execute(async (paths) => {
        const textarea = document.querySelector<HTMLTextAreaElement>('.carta-input textarea')!;
        const box = textarea.getBoundingClientRect();
        const internals = (window as unknown as { __TAURI_INTERNALS__: { invoke: (c: string, a: unknown) => Promise<unknown> } })
          .__TAURI_INTERNALS__;
        await internals.invoke('plugin:event|emit', {
          event: 'tauri://drag-drop',
          payload: { paths, position: { x: box.left + 4, y: box.top + 4 } },
        });
      }, [join(fixtures, 'tabs-137.pdf')]);
      await browser.waitUntil(
        async () => ((await byTestId('right-tab-badge-attachments').getProperty('textContent')) ?? '') === '1',
        { timeout: 10_000, timeoutMsg: 'the Attachments tab shows no count of 1' },
      );
      expect(await tab('attachments').getAttribute('aria-label')).toBe('Attachments, 1 attachment');
      // Hidden from assistive tech: the count is part of the tab's name instead.
      expect(await byTestId('right-tab-badge-attachments').getAttribute('aria-hidden')).toBe('true');
      await expectSelected('links');
    });

    it('drops the count on a page without attachments', async () => {
      await clickPageRow(OTHER);
      await browser.waitUntil(async () => (await openedTitle()).includes(OTHER), { timeout: 10_000 });
      await byTestId('right-tab-badge-attachments').waitForExist({ reverse: true, timeout: 10_000 });
      expect(await tab('attachments').getAttribute('aria-label')).toBe('Attachments');
      await clickPageRow(PAGE);
      await browser.waitUntil(async () => (await openedTitle()).includes(PAGE), { timeout: 10_000 });
    });

    it('marks the Agent tab with its status and names it', async () => {
      const mark = byTestId('right-tab-badge-agent');
      await expect(mark).toBeExisting();
      expect(await mark.getAttribute('data-status')).toBe('disconnected');
      expect(await mark.getAttribute('aria-hidden')).toBe('true');
      expect(await tab('agent').getAttribute('aria-label')).toBe('Agent, Agent not connected');
    });
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

  describe('outline on a long page', () => {
    before(async () => {
      await $('.carta-input textarea').waitForDisplayed({ timeout: 10_000 });
      await setEditorText(longPage(60));
      await browser.waitUntil(async () => (await outlineTitles()).length === 60, { timeout: 10_000 });
    });

    it('keeps the scroll position when switching tabs and back, with the strip in view', async () => {
      await browser.execute((sel) => {
        document.querySelector(sel)!.scrollTop = 300;
      }, OUTLINE_SCROLLER);
      const before = await browser.execute((sel) => document.querySelector(sel)!.scrollTop, OUTLINE_SCROLLER);
      expect(before).toBeGreaterThan(100);

      const stripInView = () =>
        browser.execute(() => document.querySelector('[data-testid="right-tabs"]')!.getBoundingClientRect().top >= 0);
      for (const other of ['links', 'attachments']) {
        await tab(other).click();
        await expectSelected(other);
        expect(await byTestId('right-tabs').isDisplayed()).toBe(true);
        expect(await stripInView()).toBe(true);
      }
      await tab('outline').click();
      await expectSelected('outline');
      expect(await browser.execute((sel) => document.querySelector(sel)!.scrollTop, OUTLINE_SCROLLER)).toBe(before);
      expect(await stripInView()).toBe(true);
    });

    it('marks the heading the user scrolled into, and follows back to the top', async () => {
      await browser.execute(() => {
        const s = document.querySelector('.carta-input') as HTMLElement;
        s.scrollTop = s.scrollHeight;
        s.dispatchEvent(new Event('scroll'));
      });
      await browser.waitUntil(async () => Number((await currentOutlineTitle())?.replace('Section ', '')) > 30, {
        timeout: 5_000,
        timeoutMsg: 'no late section was marked after scrolling down',
      });
      await browser.execute(() => {
        const s = document.querySelector('.carta-input') as HTMLElement;
        s.scrollTop = 0;
        s.dispatchEvent(new Event('scroll'));
      });
      await browser.waitUntil(async () => (await currentOutlineTitle()) === 'Section 1', {
        timeout: 5_000,
        timeoutMsg: 'Section 1 was not marked at the top',
      });
    });

    it('lists setext headings', async () => {
      await setEditorText('Setext Title\n============\n\ntext\n\nSetext Part\n-----------\n\nmore\n');
      await browser.waitUntil(async () => (await outlineTitles()).join('|') === 'Setext Title|Setext Part', {
        timeout: 5_000,
        timeoutMsg: 'setext headings were not listed',
      });
      const levels = await $$('[data-testid="outline-item"]').map((b) => b.getAttribute('data-level'));
      expect(levels).toEqual(['1', '2']);
    });

    it('jumps in preview mode', async () => {
      await setEditorText(longPage(60));
      await browser.waitUntil(async () => (await outlineTitles()).length === 60, { timeout: 10_000 });
      await browser.execute(() => (document.activeElement as HTMLElement | null)?.blur());
      await browser.keys(['Control', 'e']);
      await $('.carta-renderer').waitForDisplayed({ timeout: 10_000 });
      await browser.waitUntil(() => browser.execute(() => document.querySelectorAll('.carta-renderer h2').length === 60), {
        timeout: 10_000,
        timeoutMsg: 'the preview did not render the headings',
      });

      const items = await $$('[data-testid="outline-item"]');
      await items[44].click();
      await browser.waitUntil(
        () =>
          browser.execute(() => {
            const h = Array.from(document.querySelectorAll('.carta-renderer h2')).find((e) => e.textContent === 'Section 45')!;
            const hr = h.getBoundingClientRect();
            for (let n = h.parentElement; n; n = n.parentElement) {
              const oy = getComputedStyle(n).overflowY;
              if ((oy === 'auto' || oy === 'scroll') && n.scrollHeight > n.clientHeight) {
                const top = hr.top - n.getBoundingClientRect().top;
                return top >= -2 && top < 60;
              }
            }
            return false;
          }),
        { timeout: 5_000, timeoutMsg: 'the preview did not scroll to Section 45' },
      );
      await browser.waitUntil(async () => (await currentOutlineTitle()) === 'Section 45', {
        timeout: 5_000,
        timeoutMsg: 'Section 45 was not marked in preview mode',
      });
    });

    it('marks the heading scrolled into in preview mode', async () => {
      await browser.execute(() => {
        const h = Array.from(document.querySelectorAll('.carta-renderer h2')).find((e) => e.textContent === 'Section 10')!;
        h.scrollIntoView({ block: 'start' });
      });
      await browser.waitUntil(async () => (await currentOutlineTitle()) === 'Section 10', {
        timeout: 5_000,
        timeoutMsg: 'Section 10 was not marked after scrolling the preview',
      });
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
