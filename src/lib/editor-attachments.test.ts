import { describe, expect, it, vi } from 'vitest';

vi.mock('@tauri-apps/api/webview', () => ({ getCurrentWebview: () => ({}) }));

import { storeAndInsert, type PendingAttachment, type StoreAndInsertDeps, type TextareaLike } from './editor-attachments';

const UUID = 'f7e435db-9740-4a2a-8e57-b439c4f8bb18';

/** A textarea stand-in and the calls `storeAndInsert` made. */
function harness(value = 'hello world', pageKey = 'vault/page') {
  const textarea: TextareaLike = { value, selectionStart: value.length, selectionEnd: value.length };
  let key: string | null = pageKey;
  const errors: string[] = [];
  const inserted: { start: number; end: number; text: string }[] = [];
  let insertedCalls = 0;
  const deps: StoreAndInsertDeps<TextareaLike> = {
    getTextarea: () => textarea,
    getPageKey: () => key,
    isDisposed: () => false,
    insert: (t, start, end, text) => {
      inserted.push({ start, end, text });
      t.value = t.value.slice(0, start) + text + t.value.slice(end);
    },
    onInserted: () => insertedCalls++,
    onError: (message) => errors.push(message),
  };
  return {
    textarea,
    deps,
    errors,
    inserted,
    insertedCalls: () => insertedCalls,
    switchPage: () => (key = 'vault/other'),
  };
}

const ok = (name: string, uuid = UUID): PendingAttachment => ({ name, store: async () => ({ uuid, name }) });
const failing = (name: string): PendingAttachment => ({
  name,
  store: () => Promise.reject(new Error('Permission denied')),
});

describe('storeAndInsert', () => {
  it('inserts references at the drop point', async () => {
    const h = harness();
    await storeAndInsert([ok('photo.png'), ok('report.pdf')], { start: 6, end: 6, value: 'hello world' }, h.deps);
    expect(h.textarea.value).toBe(`hello ![photo.png](cerbo://objects/${UUID})\n[report.pdf](cerbo://objects/${UUID})world`);
    expect(h.insertedCalls()).toBe(1);
    expect(h.errors).toEqual([]);
  });

  it('reports a failed copy by name and inserts nothing', async () => {
    const h = harness();
    await storeAndInsert([failing('report.pdf')], { start: 6, end: 6, value: 'hello world' }, h.deps);
    expect(h.textarea.value).toBe('hello world');
    expect(h.inserted).toEqual([]);
    expect(h.insertedCalls()).toBe(0);
    expect(h.errors).toEqual(['Could not attach report.pdf: Permission denied']);
  });

  it('links the files that were stored and names the one that failed', async () => {
    const h = harness();
    await storeAndInsert([failing('a.pdf'), ok('b.png')], null, h.deps);
    expect(h.textarea.value).toBe(`hello world![b.png](cerbo://objects/${UUID})`);
    expect(h.errors).toEqual(['Could not attach a.pdf: Permission denied']);
  });

  it('does not insert into another page opened while storing', async () => {
    const h = harness();
    const item: PendingAttachment = {
      name: 'photo.png',
      store: async () => {
        h.switchPage();
        return { uuid: UUID, name: 'photo.png' };
      },
    };
    await storeAndInsert([item], { start: 0, end: 0, value: 'hello world' }, h.deps);
    expect(h.textarea.value).toBe('hello world');
    expect(h.insertedCalls()).toBe(0);
    expect(h.errors[0]).toMatch(/page changed/);
  });

  it('moves the drop point past text typed while storing', async () => {
    const h = harness();
    const item: PendingAttachment = {
      name: 'photo.png',
      store: async () => {
        h.textarea.value = '137 hello world';
        return { uuid: UUID, name: 'photo.png' };
      },
    };
    await storeAndInsert([item], { start: 6, end: 6, value: 'hello world' }, h.deps);
    expect(h.textarea.value).toBe(`137 hello ![photo.png](cerbo://objects/${UUID})world`);
  });

  it('falls back to the caret when the drop point was edited away', async () => {
    const h = harness();
    const item: PendingAttachment = {
      name: 'photo.png',
      store: async () => {
        h.textarea.value = 'hello';
        h.textarea.selectionStart = h.textarea.selectionEnd = 5;
        return { uuid: UUID, name: 'photo.png' };
      },
    };
    await storeAndInsert([item], { start: 6, end: 11, value: 'hello world' }, h.deps);
    expect(h.textarea.value).toBe(`hello![photo.png](cerbo://objects/${UUID})`);
  });
});
