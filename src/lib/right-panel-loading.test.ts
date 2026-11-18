import { beforeEach, describe, expect, it, vi } from 'vitest';

// Loading is tied to the Links tab being visible (change `add-right-panel-tabs`,
// spec `desktop-right-panel`, design D3): hidden means no backlinks, outgoing
// links or similar notes requests.
const invoke = vi.fn(async (cmd: string, _args?: unknown): Promise<unknown> => {
  if (cmd === 'page_read') return '# Page';
  if (cmd === 'similar_get') return { status: 'ready', notes: [] };
  return [];
});
vi.mock('@tauri-apps/api/core', () => ({ invoke: (cmd: string, args?: unknown) => invoke(cmd, args) }));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }));
vi.mock('@tauri-apps/api/path', () => ({ homeDir: vi.fn() }));
vi.mock('@tauri-apps/api/window', () => ({ getCurrentWindow: () => ({ destroy: vi.fn() }) }));
vi.mock('mode-watcher', () => ({ mode: { current: 'light' }, setMode: vi.fn() }));

const { app, openPage, loadLinksWhenVisible, loadSimilarNotesWhenVisible } = await import('./stores.svelte');

const calls = (cmd: string) => invoke.mock.calls.filter(([c]) => c === cmd).length;

describe('right panel loading gate', () => {
  beforeEach(() => {
    invoke.mockClear();
    app.activeVaultId = 'vault-137';
  });

  it('opening a page loads no backlinks or outgoing links itself', async () => {
    await openPage('page-137');
    expect(calls('page_read')).toBe(1);
    expect(calls('backlinks_get')).toBe(0);
    expect(calls('outgoing_links_get')).toBe(0);
  });

  it('loads backlinks and outgoing links once while Links is visible', async () => {
    loadLinksWhenVisible('page-137', true);
    await Promise.resolve();
    expect(calls('backlinks_get')).toBe(1);
    expect(calls('outgoing_links_get')).toBe(1);
  });

  it('loads nothing while Links is hidden or there is no page', () => {
    loadLinksWhenVisible('page-137', false);
    loadLinksWhenVisible(null, true);
    expect(calls('backlinks_get')).toBe(0);
    expect(calls('outgoing_links_get')).toBe(0);
  });

  it('requests no similar notes while another tab is selected', () => {
    loadSimilarNotesWhenVisible('page-137', false);
    loadSimilarNotesWhenVisible(undefined, true);
    expect(calls('similar_get')).toBe(0);
  });

  it('requests similar notes when Links is visible', async () => {
    loadSimilarNotesWhenVisible('page-137', true);
    await vi.waitFor(() => expect(calls('similar_get')).toBe(1));
  });
});
