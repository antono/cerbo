import { mkdirSync } from 'node:fs';
import { join } from 'node:path';
import { $, $$, browser } from '@wdio/globals';

// Edits are written 800 ms after the last one. Leaving the page sooner must
// not lose them: another page, or another vault, is opened in the same script
// turn as the edit, so the autosave delay cannot have run out.

const SECOND = 'Autosave Second';
const PAGE_MARKER = 'left-for-another-page-137';
const VAULT_MARKER = 'left-for-another-vault-137';

/** The write view's text once it starts with `# title`. */
async function waitForEditorOf(title: string) {
  await browser.waitUntil(
    () =>
      browser.execute(
        (t) => (document.querySelector('.carta-input textarea') as HTMLTextAreaElement | null)?.value.startsWith(`# ${t}`) ?? false,
        title,
      ),
    { timeout: 10_000, timeoutMsg: `write view did not show "${title}"` },
  );
}

async function openRow(title: string) {
  const uuid = await browser.execute(
    (t) =>
      Array.from(document.querySelectorAll('.page-list .item.page'))
        .find((li) => li.querySelector('.label')?.textContent === t)
        ?.getAttribute('data-uuid'),
    title,
  );
  if (!uuid) throw new Error(`no page row "${title}"`);
  await $(`.page-list .item.page[data-uuid="${uuid}"] .page-btn`).click();
}

function editorText(): Promise<string> {
  return browser.execute(() => (document.querySelector('.carta-input textarea') as HTMLTextAreaElement | null)?.value ?? '');
}

/** Append a line to the write view as an edit, then click `selector`, with no time in between. */
async function editAndClick(line: string, selector: string) {
  await browser.execute(
    (text, target) => {
      const t = document.querySelector('.carta-input textarea') as HTMLTextAreaElement;
      t.value = `${t.value}\n${text}\n`;
      t.dispatchEvent(new Event('input', { bubbles: true }));
      (document.querySelector(target) as HTMLElement).click();
    },
    line,
    selector,
  );
}

describe('edits survive leaving the page before the autosave delay', () => {
  it('opens a second page in write mode', async () => {
    await $('.page-list .page-btn').waitForDisplayed({ timeout: 30_000 });
    await browser.keys(['Control', 'n']);
    const input = $('#new-page-title');
    await input.waitForDisplayed({ timeout: 10_000 });
    await input.setValue(SECOND);
    await browser.keys('Enter');
    await input.waitForDisplayed({ reverse: true, timeout: 10_000 });
    await waitForEditorOf(SECOND);
  });

  it('keeps an edit when another page is opened at once', async () => {
    await openRow('Index');
    await waitForEditorOf('Index');
    const secondRow = await browser.execute(
      (t) =>
        Array.from(document.querySelectorAll('.page-list .item.page'))
          .find((li) => li.querySelector('.label')?.textContent === t)
          ?.getAttribute('data-uuid'),
      SECOND,
    );
    await editAndClick(PAGE_MARKER, `.page-list .item.page[data-uuid="${secondRow}"] .page-btn`);
    await waitForEditorOf(SECOND);

    // Opening a page reads it from disk: the marker is there only if it was written.
    await openRow('Index');
    await waitForEditorOf('Index');
    await browser.waitUntil(async () => (await editorText()).includes(PAGE_MARKER), {
      timeout: 5_000,
      timeoutMsg: 'the edit made just before opening another page was not saved',
    });
  });

  it('keeps an edit when another vault is opened at once', async () => {
    await $('[data-testid="toolbar-vault"]').click();
    const rows = $$('.vault-selector-modal .vault-item');
    await rows[1].waitForExist({ timeout: 10_000 });
    // session.ts registers the second vault as a bare folder; make it a vault
    // the app can open. The row's tooltip is its path.
    const otherVault = await rows[1].getAttribute('title');
    mkdirSync(join(otherVault, '.cerbo', 'objects'), { recursive: true });
    // The selector is open over the editor; edit and pick the other vault in one turn.
    await editAndClick(VAULT_MARKER, '.vault-selector-modal .vault-item:not(.selected)');
    await $('.vault-selector-modal').waitForExist({ reverse: true, timeout: 10_000 });
    // The other vault is empty: it opens on a fresh Index page without the marker.
    await browser.waitUntil(async () => {
      const text = await editorText();
      return text.startsWith('# Index') && !text.includes(VAULT_MARKER);
    }, {
      timeout: 30_000,
      timeoutMsg: 'the other vault did not open',
    });

    await $('[data-testid="toolbar-vault"]').click();
    await rows[1].waitForExist({ timeout: 10_000 });
    await $('.vault-selector-modal .vault-item:not(.selected)').click();
    await $('.vault-selector-modal').waitForExist({ reverse: true, timeout: 10_000 });
    await browser.waitUntil(async () => (await editorText()).includes(VAULT_MARKER), {
      timeout: 30_000,
      timeoutMsg: 'the edit made just before opening another vault was not saved',
    });
  });
});
