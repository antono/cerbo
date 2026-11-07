import { $, browser, expect } from '@wdio/globals';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

// Spec `editor-attachment-drop`: a dropped file, a pasted image and a dropped
// text fragment become Attachment objects, their references land in page.md,
// and the page's attachments panel lists them; a failed copy inserts nothing.
//
// WebDriver cannot drive a native drag or the system clipboard. OS file drops
// are replayed as the `tauri://drag-drop` event Tauri emits for them; paste
// and fragment drops are dispatched on the textarea with a DataTransfer
// stand-in. Everything after the event (copy, insertion, autosave) is the
// real app.

const REFERENCE = /(!?)\[([^\]]+)\]\(cerbo:\/\/objects\/([0-9a-f-]+)\)/g;

/** References in the editor's textarea. */
function editorReferences(): Promise<{ image: boolean; name: string; uuid: string }[]> {
  return browser.execute((source) => {
    const value = document.querySelector<HTMLTextAreaElement>('.carta-input textarea')?.value ?? '';
    return Array.from(value.matchAll(new RegExp(source, 'g')), (m) => ({ image: m[1] === '!', name: m[2], uuid: m[3] }));
  }, REFERENCE.source);
}

function invoke<T>(cmd: string, args: Record<string, unknown>): Promise<T> {
  return browser.execute(
    (c, a) => (window as unknown as { __TAURI_INTERNALS__: { invoke: (c: string, a: unknown) => Promise<unknown> } }).__TAURI_INTERNALS__.invoke(c, a),
    cmd,
    args,
  ) as Promise<T>;
}

function currentUuid(): Promise<string | null> {
  return browser.execute(() => document.querySelector('.page-list .item.page.active')?.getAttribute('data-uuid') ?? null);
}

/** Wait until page.md on disk holds `reference` and the page's attachments list `uuid`. */
async function expectSavedAndListed(page: string, reference: string, uuid: string) {
  await browser.waitUntil(async () => (await invoke<string>('page_read', { uuid: page })).includes(reference), {
    timeout: 10_000,
    timeoutMsg: `page.md never contained ${reference}`,
  });
  expect(await invoke<string[]>('attachment_list', { uuid: page })).toContain(uuid);
  await browser.waitUntil(
    async () =>
      (await browser.execute(
        (u) => Array.from(document.querySelectorAll('.attachments-panel .filename'), (el) => el.textContent).includes(u),
        uuid,
      )) === true,
    { timeout: 10_000, timeoutMsg: 'the attachments panel does not list the new object' },
  );
}

/** Replay an OS file drop of `paths` at the textarea's top-left text. */
async function dropPaths(paths: string[]) {
  await browser.execute(async (p) => {
    const textarea = document.querySelector<HTMLTextAreaElement>('.carta-input textarea')!;
    const box = textarea.getBoundingClientRect();
    // On Linux the drop position is in CSS pixels, as GTK reports it.
    const position = { x: box.left + 4, y: box.top + 4 };
    const internals = (window as unknown as { __TAURI_INTERNALS__: { invoke: (c: string, a: unknown) => Promise<unknown> } })
      .__TAURI_INTERNALS__;
    await internals.invoke('plugin:event|emit', { event: 'tauri://drag-drop', payload: { paths: p, position } });
  }, paths);
}

function editorValue(): Promise<string> {
  return browser.execute(() => document.querySelector<HTMLTextAreaElement>('.carta-input textarea')?.value ?? '');
}

describe('drop and paste into the editor', () => {
  let page: string;
  let fixtures: string;

  before(async () => {
    fixtures = mkdtempSync(join(tmpdir(), 'cerbo-drop-'));
    writeFileSync(join(fixtures, 'report.pdf'), '%PDF-1.4\n% 137\n%%EOF\n');

    const row = $('.page-list .page-btn');
    await row.waitForDisplayed({ timeout: 30_000 });
    await row.click();
    await browser.waitUntil(async () => !!(await currentUuid()), { timeout: 10_000 });
    page = (await currentUuid())!;
    // The attachments panel lives in the right sidebar.
    const showPanels = $('.show-backlinks-btn');
    if (await showPanels.isExisting()) await showPanels.click();
    await $('.attachments-panel').waitForDisplayed({ timeout: 10_000 });
    await browser.execute(() => (document.activeElement as HTMLElement | null)?.blur());
    await browser.keys('i');
    await $('.carta-input textarea').waitForDisplayed({ timeout: 10_000 });
  });

  after(() => {
    if (fixtures) rmSync(fixtures, { recursive: true, force: true });
  });

  it('drops a file from the file manager as an Attachment', async () => {
    const source = join(fixtures, 'report.pdf');
    await dropPaths([source]);

    await browser.waitUntil(async () => (await editorReferences()).some((r) => r.name === 'report.pdf'), {
      timeout: 10_000,
      timeoutMsg: 'no reference was inserted for the dropped file',
    });
    const ref = (await editorReferences()).find((r) => r.name === 'report.pdf')!;
    expect(ref.image).toBe(false);
    await expectSavedAndListed(page, `[report.pdf](cerbo://objects/${ref.uuid})`, ref.uuid);
  });

  it('reports a failed copy and inserts no reference', async () => {
    await browser.execute(() => {
      // Clear an earlier toast so the one below is this drop's.
      document.querySelector<HTMLButtonElement>('.error-toast .toast-close')?.click();
    });
    const before = await editorValue();
    await dropPaths([join(fixtures, 'missing-137.pdf')]);

    const toast = $('.error-toast');
    await toast.waitForDisplayed({ timeout: 10_000 });
    expect(await toast.getText()).toContain('missing-137.pdf');
    expect(await editorValue()).toBe(before);
    expect((await editorReferences()).some((r) => r.name === 'missing-137.pdf')).toBe(false);
    await browser.execute(() => document.querySelector<HTMLButtonElement>('.error-toast .toast-close')?.click());
  });

  it('pastes an image as an Attachment at the caret', async () => {
    await browser.execute(() => {
      const textarea = document.querySelector<HTMLTextAreaElement>('.carta-input textarea')!;
      textarea.focus();
      textarea.setSelectionRange(textarea.value.length, textarea.value.length);
      // A 1×1 PNG.
      const png = Uint8Array.from(
        atob('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg=='),
        (c) => c.charCodeAt(0),
      );
      const file = new File([png], 'fixture.png', { type: 'image/png' });
      const event = new Event('paste', { bubbles: true, cancelable: true });
      Object.defineProperty(event, 'clipboardData', {
        value: { types: ['Files', 'text/html'], files: [file], getData: (t: string) => (t === 'text/html' ? '<img>' : '') },
      });
      textarea.dispatchEvent(event);
    });

    await browser.waitUntil(async () => (await editorReferences()).some((r) => r.name === 'fixture.png'), {
      timeout: 10_000,
      timeoutMsg: 'no reference was inserted for the pasted image',
    });
    const ref = (await editorReferences()).find((r) => r.name === 'fixture.png')!;
    expect(ref.image).toBe(true);
    await expectSavedAndListed(page, `![fixture.png](cerbo://objects/${ref.uuid})`, ref.uuid);

    // The preview shows the image from the object's file under `.cerbo/`.
    await $('.mode-toggle-btn').click();
    await browser.waitUntil(
      async () =>
        (await browser.execute((u) => {
          const img = document.querySelector<HTMLImageElement>(`img[data-cerbo-uuid="${u}"]`);
          return !!img && img.complete && img.naturalWidth > 0;
        }, ref.uuid)) === true,
      { timeout: 10_000, timeoutMsg: 'the preview did not load the pasted image' },
    );
    await $('.mode-toggle-btn').click();
    await $('.carta-input textarea').waitForDisplayed({ timeout: 10_000 });
  });

  it('drops a text fragment as a .txt Attachment', async () => {
    await browser.execute(() => {
      const textarea = document.querySelector<HTMLTextAreaElement>('.carta-input textarea')!;
      const box = textarea.getBoundingClientRect();
      const event = new Event('drop', { bubbles: true, cancelable: true });
      Object.defineProperties(event, {
        clientX: { value: box.left + 4 },
        clientY: { value: box.top + 4 },
        dataTransfer: {
          value: { types: ['text/plain'], files: [], getData: (t: string) => (t === 'text/plain' ? 'one hundred thirty-seven' : '') },
        },
      });
      textarea.dispatchEvent(event);
    });

    await browser.waitUntil(async () => (await editorReferences()).some((r) => r.name.endsWith('.txt')), {
      timeout: 10_000,
      timeoutMsg: 'no reference was inserted for the dropped text',
    });
    const ref = (await editorReferences()).find((r) => r.name.endsWith('.txt'))!;
    expect(ref.image).toBe(false);
    await expectSavedAndListed(page, `[${ref.name}](cerbo://objects/${ref.uuid})`, ref.uuid);
  });
});
