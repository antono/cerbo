/**
 * Cerbo app state — Svelte 5 reactive store.
 *
 * Central hub for vault, page, and UI state.
 * All Tauri commands are invoked from here, keeping components thin.
 */

import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { homeDir } from '@tauri-apps/api/path';
import { backlinksFailure, type BacklinksStatus } from './backlinks';
import type { PrefixMap } from './mention-plugin';
import type { SparqlResponse } from './sparql-console';
import {
  enableFailed,
  enableStarted,
  enableSucceeded,
  initialSimilarState,
  isSuperseded,
  progressReceived,
  similarFailed,
  similarLoaded,
  similarRequested,
  type SemanticProgress,
  type SimilarNotesResponse,
  type SimilarState,
} from './similar-notes';
import { serialQueue } from './write-queue';

// ── Types (mirror Rust structs) ───────────────────────────────────────────────

export interface Vault {
  id: string;
  name: string;
  path: string;
  lastOpenPage: string | null;
}

interface AppStateFile {
  activeVaultId: string | null;
  vaultStates: Record<string, { lastOpenPage: string | null }>;
}

export interface PageMeta {
  uuid: string;
  title: string;
}

export interface BacklinkEntry {
  uuid: string;
  title: string;
}

export interface VaultObject {
  uuid: string;
  title: string;
  object_type: string;
}

export interface VaultsFile {
  vaults: Vault[];
  activeVaultId: string | null;
}

export interface UiSettings {
  theme: ThemeMode | null;
  fontSize: number | null;
  sidebarWidth: number | null;
  rightSidebarVisible: boolean | null;
  windowBounds: { width: number; height: number } | null;
  lastSparqlQuery: string | null;
}

export type ThemeMode = 'light' | 'dark' | 'system';

// ── State ─────────────────────────────────────────────────────────────────────

export const app = $state({
  vaults: [] as Vault[],
  activeVaultId: null as string | null,
  pages: [] as PageMeta[],
  vaultObjects: [] as VaultObject[],
  // The open vault's `prefix → namespace` map for mentions; null until loaded.
  prefixMap: null as PrefixMap | null,
  currentUuid: null as string | null,
  currentContent: '',
  backlinks: [] as BacklinkEntry[],
  backlinksStatus: 'ready' as BacklinksStatus,
  backlinksMessage: '',
  attachments: [] as string[],
  loading: false,
  loadingMessage: '',
  error: null as string | null,

  // History for navigation
  history: [] as string[],
  historyIndex: -1,

  // Layout state
  sidebarWidth: 260,
  backlinksWidth: 280,
  showRightSidebar: true,

  // UI state
  theme: 'light' as ThemeMode,
  editorTab: 'preview' as 'write' | 'preview',
  showSearch: false,
  showExitPrompt: false,
  showNewPageForm: false,
  showVaultSelector: false,
  showHelp: false,
  renameUuid: null as string | null,
  renameTitle: '',
  confirmDeleteUuid: null as string | null,
  showSparqlConsole: false,
  // Last query run in the SPARQL console, kept in ui.toml across restarts.
  lastSparqlQuery: null as string | null,
  // The "Similar notes" sidebar section; see similar-notes.ts.
  similar: { ...initialSimilarState } as SimilarState,
});

export async function loadUiSettings(): Promise<void> {
  try {
    const res = await invoke<UiSettings>('ui_settings_load');
    if (res.theme) app.theme = res.theme;
    if (typeof res.fontSize === 'number') {
      // TODO: apply font size in editor UI when settings are exposed.
    }
    if (typeof res.sidebarWidth === 'number') {
      app.sidebarWidth = res.sidebarWidth;
    }
    if (typeof res.rightSidebarVisible === 'boolean') {
      app.showRightSidebar = res.rightSidebarVisible;
    }
    if (typeof res.lastSparqlQuery === 'string') {
      app.lastSparqlQuery = res.lastSparqlQuery;
    }
    // `windowBounds` is applied by the desktop layer when the window opens
    // and saved by `app_exit`; the frontend never sends it (null keeps it).
  } catch (e) {
    setError(String(e));
  }
}

export async function saveUiSettings(): Promise<void> {
  try {
    await invoke('ui_settings_save', {
      theme: app.theme,
      fontSize: null,
      sidebarWidth: app.sidebarWidth,
      rightSidebarVisible: app.showRightSidebar,
      windowBounds: null, // kept: `app_exit` saves the window size
      lastSparqlQuery: app.lastSparqlQuery,
    });
  } catch (e) {
    console.error('Failed to save UI settings:', e);
  }
}

/**
 * Closes all transient UI elements (modals, forms, switchers).
 * Use this before opening a new one to prevent interference.
 */
export function closeAllDialogs() {
  app.showSearch = false;
  app.showExitPrompt = false;
  app.showNewPageForm = false;
  app.showVaultSelector = false;
  app.showHelp = false;
  app.renameUuid = null;
  app.renameTitle = '';
  app.confirmDeleteUuid = null;
  app.showSparqlConsole = false;
}

// ── Computed helpers ──────────────────────────────────────────────────────────

export function activeVault(): Vault | undefined {
  return app.vaults.find((v) => v.id === app.activeVaultId);
}

export function pageUuids(): string[] {
  return app.pages.map((p) => p.uuid);
}

export function pageTitles(): string[] {
  return app.pages.map((p) => p.title);
}

// ── Commands ──────────────────────────────────────────────────────────────────

export async function quitApp(): Promise<void> {
  try {
    await saveUiSettings();
    await invoke('app_exit');
  } catch (e) {
    console.error('Failed to quit app:', e);
    // Last resort
    const { getCurrentWindow } = await import('@tauri-apps/api/window');
    await getCurrentWindow().destroy();
  }
}

export async function loadVaults(): Promise<void> {
  try {
    const res = await invoke<VaultsFile>('vault_list');
    app.vaults = res.vaults;

    const state = await invoke<AppStateFile>('state_load');
    for (const vault of app.vaults) {
      vault.lastOpenPage = state.vaultStates[vault.id]?.lastOpenPage ?? null;
    }

    app.activeVaultId = state.activeVaultId ?? res.activeVaultId;

    // Fallback if activeVaultId is invalid
    if (app.activeVaultId && !app.vaults.find((v) => v.id === app.activeVaultId)) {
      app.activeVaultId = null;
    }
    if (!app.activeVaultId && app.vaults.length > 0) {
      app.activeVaultId = app.vaults[0].id;
    }
  } catch (e) {
    setError(String(e));
  }
}

export async function openVault(vaultId: string): Promise<void> {
  app.loading = true;
  app.loadingMessage = 'Opening vault…';
  app.error = null;
  try {
    await invoke('vault_open', { vaultId: vaultId });
    app.activeVaultId = vaultId;
    await invoke('vault_set_active', { id: vaultId });
    await loadPages();
    await loadVaultObjects();
    // Before the first page renders, so mentions resolve on first paint.
    await loadPrefixMap();

    const vault = activeVault();
    if (!vault) return;

    // Create Index page if vault is completely empty
    if (app.pages.length === 0) {
      await createPage('Index');
      return;
    }

    // Restore last-open page
    const last = vault.lastOpenPage;
    if (last && app.pages.find((p) => p.uuid === last)) {
      await openPage(last);
    } else if (app.pages.length > 0) {
      const indexPage = app.pages.find((p) => p.title === 'Index');
      await openPage(indexPage ? indexPage.uuid : app.pages[0].uuid);
    } else {
      app.currentUuid = null;
      app.currentContent = '';
      app.backlinks = [];
    }
  } catch (e) {
    setError(String(e));
  } finally {
    app.loading = false;
  }
}

export async function addVault(name: string, path: string): Promise<void> {
  try {
    await invoke('vault_add', { name, path });
    await loadVaults();
    const added = app.vaults.find((v) => v.name === name && v.path === path);
    if (added) await openVault(added.id);
  } catch (e) {
    setError(String(e));
    throw e;
  }
}

export async function quickAddVault(): Promise<void> {
  try {
    const selected = await open({
      directory: true,
      multiple: false,
      defaultPath: await homeDir(),
    });
    if (selected && typeof selected === 'string') {
      app.loading = true;
      app.loadingMessage = 'Adding vault...';
      const parts = selected.split(/[\\/]/);
      const last = parts.pop() || parts.pop(); // handle trailing slash
      const name = last || 'New Vault';
      
      await addVault(name, selected);
      closeAllDialogs();
    }
  } catch (e) {
    console.error('Failed to add vault', e);
    setError(String(e));
  } finally {
    app.loading = false;
  }
}

export async function loadPages(): Promise<void> {
  if (!app.activeVaultId) return;
  try {
    const pages = await invoke<PageMeta[]>('page_list');
    app.pages = pages;
  } catch (e) {
    setError(String(e));
  }
}

export async function loadPrefixMap(): Promise<void> {
  app.prefixMap = null;
  if (!app.activeVaultId) return;
  try {
    app.prefixMap = await invoke<PrefixMap>('prefix_map_get');
  } catch (e) {
    // Mentions still render, only without types; opening the vault goes on.
    console.error('Failed to load prefix map:', e);
  }
}

/**
 * Run a read-only SPARQL query on the open vault (the console). The query is
 * remembered even when it fails, so a typo is not lost on reopen. Rejects with
 * the command's `{ kind, message? }` error.
 */
export async function runSparqlQuery(query: string): Promise<SparqlResponse> {
  app.lastSparqlQuery = query;
  void saveUiSettings();
  return invoke<SparqlResponse>('sparql_query', { query });
}

export async function loadVaultObjects(): Promise<void> {
  if (!app.activeVaultId) return;
  try {
    const objects = await invoke<VaultObject[]>('vault_objects_list', { vaultId: app.activeVaultId });
    app.vaultObjects = objects;
  } catch (e) {
    setError(String(e));
  }
}

export interface OpenPageOptions {
  pushToHistory?: boolean;
}

export async function openPage(uuid: string, options?: OpenPageOptions): Promise<void> {
  if (!app.activeVaultId) return;
  const pushToHistory = options?.pushToHistory ?? true;

  // Handle history
  if (pushToHistory) {
    if (app.historyIndex < app.history.length - 1) {
      app.history = app.history.slice(0, app.historyIndex + 1);
    }
    if (app.history[app.history.length - 1] !== uuid) {
      app.history.push(uuid);
      app.historyIndex = app.history.length - 1;
    }
  }

  try {
    const content = await invoke<string>('page_read', { uuid });
    app.currentUuid = uuid;
    app.currentContent = content;
    await invoke('vault_update_last_page', { vaultId: app.activeVaultId, uuid });
    const v = activeVault();
    if (v) v.lastOpenPage = uuid;

    await loadBacklinks(uuid);
    await loadAttachments(uuid);
  } catch (e) {
    setError(String(e));
  }
}

export async function openNextPage(): Promise<void> {
  if (app.pages.length === 0) return;
  const currentIndex = app.pages.findIndex(p => p.uuid === app.currentUuid);
  const nextIndex = (currentIndex + 1) % app.pages.length;
  await openPage(app.pages[nextIndex].uuid);
}

export async function openPrevPage(): Promise<void> {
  if (app.pages.length === 0) return;
  const currentIndex = app.pages.findIndex(p => p.uuid === app.currentUuid);
  const prevIndex = (currentIndex - 1 + app.pages.length) % app.pages.length;
  await openPage(app.pages[prevIndex].uuid);
}

// Page changes run off the main thread in the desktop layer; queue them so they
// reach the vault in the order they were made.
const vaultWrite = serialQueue();

export async function savePage(uuid: string, content: string): Promise<string | undefined> {
  if (!app.activeVaultId) return;
  try {
    const finalContent = await vaultWrite(() => invoke<string>('page_write', { uuid, content }));
    return finalContent;
  } catch (e) {
    setError(String(e));
  }
}

export async function createPage(title: string, slug?: string, virtualPath?: string): Promise<string> {
  if (!app.activeVaultId) throw new Error('No active vault');
  const uuid = await vaultWrite(() => invoke<string>('page_create', { title, slug, virtualPath }));
  await loadPages();
  app.editorTab = 'write';
  await openPage(uuid);
  return uuid;
}

export async function deletePage(uuid: string): Promise<void> {
  if (!app.activeVaultId) return;
  try {
    await vaultWrite(() => invoke('page_delete', { uuid }));
    await loadPages();
    if (app.currentUuid === uuid) {
      if (app.pages.length > 0) {
        await openPage(app.pages[0].uuid);
      } else {
        app.currentUuid = null;
        app.currentContent = '';
        app.backlinks = [];
      }
    }
  } catch (e) {
    setError(String(e));
    throw e;
  }
}

export async function updatePageTitle(uuid: string, newTitle: string): Promise<void> {
  if (!app.activeVaultId) throw new Error('No active vault');
  await vaultWrite(() => invoke('page_update_title', { uuid, newTitle }));
  await loadPages();
  await openPage(uuid);
}

export function triggerRename(uuid?: string) {
  const targetUuid = uuid || app.currentUuid;
  if (!targetUuid) return;
  const page = app.pages.find(p => p.uuid === targetUuid);
  if (!page) return;

  closeAllDialogs();
  app.renameUuid = targetUuid;
  app.renameTitle = page.title;
}

export function triggerDelete(uuid?: string) {
  const targetUuid = uuid || app.currentUuid;
  if (!targetUuid) return;
  closeAllDialogs();
  app.confirmDeleteUuid = targetUuid;
}

export function extractTitle(content: string): string | null {
  const lines = content.split('\n');
  for (const line of lines) {
    if (line.trim().startsWith('# ')) {
      return line.trim().slice(2).trim();
    }
  }
  return null;
}

// Only the latest request may update the panel; an older one can finish later.
let backlinksRequest = 0;

export async function loadBacklinks(uuid: string): Promise<void> {
  if (!app.activeVaultId) return;
  const request = ++backlinksRequest;
  app.backlinksStatus = 'loading';
  app.backlinksMessage = '';
  try {
    const entries = await invoke<BacklinkEntry[]>('backlinks_get', {
      vaultId: app.activeVaultId,
      uuid,
    });
    if (request !== backlinksRequest) return;
    app.backlinks = entries;
    app.backlinksStatus = 'ready';
  } catch (e) {
    if (request !== backlinksRequest) return;
    // The page stays open and editable; only the panel shows the failure.
    const failure = backlinksFailure(e);
    app.backlinks = [];
    app.backlinksStatus = failure.status;
    app.backlinksMessage = failure.message;
  }
}

/**
 * Load the similar notes of page `uuid`. An answer is kept only while `uuid`
 * is still the open page (design D2); while embeddings are being enabled the
 * list loads once that ends.
 */
export async function loadSimilarNotes(uuid: string): Promise<void> {
  if (!app.activeVaultId) return;
  app.similar = similarRequested(app.similar, uuid);
  if (app.similar.status === 'enabling') return;
  await fetchSimilarNotes(uuid);
}

async function fetchSimilarNotes(uuid: string): Promise<void> {
  try {
    const response = await invoke<SimilarNotesResponse>('similar_get', { uuid });
    app.similar = similarLoaded(app.similar, app.currentUuid, response);
  } catch (e) {
    // A newer request replaced this one before it ran; that one answers.
    if (isSuperseded(e)) return;
    // The page stays open and editable; only the section shows the failure.
    app.similar = similarFailed(app.similar, app.currentUuid, uuid, e);
  }
}

/**
 * Compute the vault's embeddings ("Enable" / "Retry"), the same work as
 * `cerbo index --embeddings`, then load the open page's similar notes.
 */
export async function enableEmbeddings(): Promise<void> {
  if (!app.activeVaultId || app.similar.status === 'enabling') return;
  app.similar = enableStarted(app.similar);
  try {
    await invoke('embeddings_refresh');
  } catch (e) {
    app.similar = enableFailed(app.similar, e);
    return;
  }
  app.similar = enableSucceeded(app.similar, app.currentUuid);
  if (app.similar.uuid) await fetchSimilarNotes(app.similar.uuid);
}

/** Show a `semantic://progress` step in the similar notes section. */
export function applySemanticProgress(progress: SemanticProgress): void {
  app.similar = progressReceived(app.similar, progress);
}

export async function goBack(): Promise<void> {
  if (app.historyIndex <= 0) return;
  app.historyIndex--;
  const slug = app.history[app.historyIndex];
  if (slug) await openPage(slug, { pushToHistory: false });
}

export async function goForward(): Promise<void> {
  if (app.historyIndex >= app.history.length - 1) return;
  app.historyIndex++;
  const slug = app.history[app.historyIndex];
  if (slug) await openPage(slug, { pushToHistory: false });
}

export async function loadAttachments(uuid: string): Promise<void> {
  if (!app.activeVaultId || !uuid) return;
  try {
    const attachments = await invoke<string[]>('attachment_list', {
      vaultId: app.activeVaultId,
      uuid,
    });
    app.attachments = attachments;
  } catch (e) {
    console.error('Failed to load attachments:', e);
    app.attachments = [];
  }
}

// ── Error handling ────────────────────────────────────────────────────────────

export function setError(msg: string) {
  app.error = msg;
  setTimeout(() => {
    app.error = null;
  }, 5000);
}

export function clearError() {
  app.error = null;
}
