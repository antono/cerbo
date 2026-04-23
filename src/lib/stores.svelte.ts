/**
 * Cerbo app state — Svelte 5 reactive store.
 *
 * Central hub for vault, page, and UI state.
 * All Tauri commands are invoked from here, keeping components thin.
 */

import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { homeDir } from '@tauri-apps/api/path';

// ── Types (mirror Rust structs) ───────────────────────────────────────────────

export interface Vault {
  id: string;
  name: string;
  path: string;
  lastOpenPage: string | null;
}

export interface PageMeta {
  slug: string;
  title: string;
}

export interface BacklinkEntry {
  slug: string;
  title: string;
}

export interface VaultsFile {
  vaults: Vault[];
  activeVaultId: string | null;
}

// ── State ─────────────────────────────────────────────────────────────────────

export const app = $state({
  vaults: [] as Vault[],
  activeVaultId: null as string | null,
  pages: [] as PageMeta[],
  currentSlug: null as string | null,
  currentContent: '',
  backlinks: [] as BacklinkEntry[],
  attachments: [] as string[],
  loading: false,
  loadingMessage: '',
  error: null as string | null,

  // Layout state
  sidebarWidth: 260,
  backlinksWidth: 280,
  backlinksVisible: true,

  // UI state
  editorTab: 'preview' as 'write' | 'preview',
  showSearch: false,
  showExitPrompt: false,
  showNewPageForm: false,
  showVaultSwitcher: false,
  showHelp: false,
  renameSlug: null as string | null,
  renameTitle: '',
  confirmDeleteSlug: null as string | null,
});

/**
 * Closes all transient UI elements (modals, forms, switchers).
 * Use this before opening a new one to prevent interference.
 */
export function closeAllDialogs() {
  app.showSearch = false;
  app.showExitPrompt = false;
  app.showNewPageForm = false;
  app.showVaultSwitcher = false;
  app.showHelp = false;
  app.renameSlug = null;
  app.renameTitle = '';
  app.confirmDeleteSlug = null;
}

// ── Computed helpers ──────────────────────────────────────────────────────────

export function activeVault(): Vault | undefined {
  return app.vaults.find((v) => v.id === app.activeVaultId);
}

export function pageSlugs(): string[] {
  return app.pages.map((p) => p.slug);
}

// ── Commands ──────────────────────────────────────────────────────────────────

export async function quitApp(): Promise<void> {
  try {
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
    app.activeVaultId = res.activeVaultId;

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

    const vault = activeVault();
    if (!vault) return;

    // Create Index page if vault is completely empty
    if (app.pages.length === 0) {
      await createPage('Index');
      return;
    }

    // Restore last-open page
    const last = vault.lastOpenPage;
    if (last && app.pages.find((p) => p.slug === last)) {
      await openPage(last);
    } else if (app.pages.length > 0) {
      // Try to find 'index' slug first, otherwise first available
      const indexPage = app.pages.find((p) => p.slug === 'index');
      await openPage(indexPage ? indexPage.slug : app.pages[0].slug);
    } else {
      app.currentSlug = null;
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
    const pages = await invoke<PageMeta[]>('page_list', { vaultId: app.activeVaultId });
    app.pages = pages;
  } catch (e) {
    setError(String(e));
  }
}

export async function openPage(slug: string): Promise<void> {
  if (!app.activeVaultId) return;
  try {
    const content = await invoke<string>('page_read', { vaultId: app.activeVaultId, slug });
    app.currentSlug = slug;
    app.currentContent = content;
    await invoke('vault_update_last_page', { vaultId: app.activeVaultId, slug });
    // Update local state too so we don't have to reload all vaults
    const v = activeVault();
    if (v) v.lastOpenPage = slug;
    
    await loadBacklinks(slug);
    await loadAttachments(slug);
  } catch (e) {
    setError(String(e));
  }
}

export async function openNextPage(): Promise<void> {
  if (app.pages.length === 0) return;
  const currentIndex = app.pages.findIndex(p => p.slug === app.currentSlug);
  const nextIndex = (currentIndex + 1) % app.pages.length;
  await openPage(app.pages[nextIndex].slug);
}

export async function openPrevPage(): Promise<void> {
  if (app.pages.length === 0) return;
  const currentIndex = app.pages.findIndex(p => p.slug === app.currentSlug);
  const prevIndex = (currentIndex - 1 + app.pages.length) % app.pages.length;
  await openPage(app.pages[prevIndex].slug);
}

export async function savePage(slug: string, content: string): Promise<void> {
  if (!app.activeVaultId) return;
  try {
    await invoke('page_write', { vaultId: app.activeVaultId, slug, content });
  } catch (e) {
    setError(String(e));
  }
}

export async function createPage(title: string): Promise<string> {
  if (!app.activeVaultId) throw new Error('No active vault');
  const slug = await invoke<string>('page_create', { vaultId: app.activeVaultId, title });
  await loadPages();
  app.editorTab = 'write';
  await openPage(slug);
  return slug;
}

export async function deletePage(slug: string): Promise<void> {
  if (!app.activeVaultId) return;
  try {
    await invoke('page_delete', { vaultId: app.activeVaultId, slug });
    await loadPages();
    if (app.currentSlug === slug) {
      if (app.pages.length > 0) {
        await openPage(app.pages[0].slug);
      } else {
        app.currentSlug = null;
        app.currentContent = '';
        app.backlinks = [];
      }
    }
  } catch (e) {
    setError(String(e));
    throw e;
  }
}

export async function renamePage(oldSlug: string, newTitle: string): Promise<string> {
  if (!app.activeVaultId) throw new Error('No active vault');
  const newSlug = await invoke<string>('page_rename', {
    vaultId: app.activeVaultId,
    oldSlug: oldSlug,
    newTitle: newTitle,
  });
  await loadPages();
  if (app.currentSlug === oldSlug) {
    await openPage(newSlug);
  }
  return newSlug;
}

export function triggerRename(slug?: string) {
  const targetSlug = slug || app.currentSlug;
  if (!targetSlug) return;
  const page = app.pages.find(p => p.slug === targetSlug);
  if (!page) return;

  closeAllDialogs();
  app.renameSlug = targetSlug;
  app.renameTitle = page.title;
}

export function triggerDelete(slug?: string) {
  const targetSlug = slug || app.currentSlug;
  if (!targetSlug) return;
  closeAllDialogs();
  app.confirmDeleteSlug = targetSlug;
}

export async function previewSlug(title: string): Promise<string> {
  return invoke<string>('slug_from_title', { title });
}

export async function loadBacklinks(slug: string): Promise<void> {
  if (!app.activeVaultId) return;
  try {
    const entries = await invoke<BacklinkEntry[]>('backlinks_get', {
      vaultId: app.activeVaultId,
      slug,
    });
    app.backlinks = entries;
  } catch (_) {
    app.backlinks = [];
  }
}

export async function loadAttachments(slug: string): Promise<void> {
  if (!app.activeVaultId || !slug) return;
  try {
    const attachments = await invoke<string[]>('attachment_list', {
      vaultId: app.activeVaultId,
      slug,
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
