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
import { parseRightPanel, tabShortcutOutcome, type AgentStatus, type RightPanelId } from './right-panels';
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
import {
  changeWidthClass,
  dockedWidths,
  panelShown,
  showPanel,
  togglePanel,
  widthClassFor,
  type Overlay,
  type Panel,
  type PanelState,
  type WidthClass,
} from './layout';
import {
  ancestorFolders,
  buildPageTree,
  carryCollapsed,
  EMPTY_LAYOUT,
  flattenVisible,
  pruneCollapsed,
  revealPath,
  visiblePageUuids,
  type Placement,
  type TreeLayout,
  type TreeNode,
  type TreeRow,
} from './page-tree';
import {
  appendPage,
  buildViewTree,
  moveAt,
  removeAt,
  type StoredNode,
  type ViewData,
  type ViewMeta,
} from './view-tree';


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
  /** The page's `cerbo:virtualPath`; null at the root. */
  virtualPath: string | null;
}

export interface BacklinkEntry {
  uuid: string;
  title: string;
}

export interface VaultObject {
  uuid: string;
  title: string;
  /** Serialized from the Rust `object_type` field (camelCase). */
  objectType: string;
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
  /** Page tree docked in the wide layout; absent in old files (= shown). */
  leftSidebarVisible?: boolean | null;
  windowBounds: { width: number; height: number } | null;
  lastSparqlQuery: string | null;
  /** Selected right panel tab; absent in old files (= Links). */
  rightPanelTab?: string | null;
  /** Collapsed sidebar folders per vault id; only collapsed ones are listed. */
  collapsedFolders?: Record<string, string[]>;
  /** The sidebar view chosen per vault id; absent = the default view. */
  activeViews?: Record<string, string>;
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
  // Outgoing links of the open page (Links sub-panel), loaded like backlinks.
  outgoingLinks: [] as BacklinkEntry[],
  outgoingStatus: 'ready' as BacklinksStatus,
  outgoingMessage: '',
  // The selected right panel tab, kept in ui.toml (spec `desktop-right-panel`).
  rightPanelTab: 'links' as RightPanelId,
  // The Agent tab's status mark; `add-agent-panel` drives it.
  agentStatus: 'disconnected' as AgentStatus,
  // The open page's text as typed, not yet saved; the Outline reads it. Only
  // trusted while `pageTextUuid` is the open page.
  pageText: '',
  pageTextUuid: null as string | null,
  // Index of the Outline heading holding the top of the editor, -1 for none.
  outlineCurrent: -1,
  attachments: [] as AttachmentInfo[],
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
  // Page tree docked (spec `desktop-window-toolbar`, "Panel toggles").
  showLeftSidebar: true,
  // Layout for the current window width (spec `desktop-responsive-layout`),
  // set from a ResizeObserver in +layout.svelte.
  widthClass: 'wide' as WidthClass,
  // Window width in px from the layout's ResizeObserver; 0 until measured.
  // Docked panel widths are clamped against it (`dockedWidths`).
  windowWidth: 0,
  // The panel floating over the editor (medium right panel, narrow drawer or
  // sheet). Never persisted; the docked flags above stay as they were.
  overlay: null as Overlay,
  // Folder picked from a toolbar path segment; the tree selects and focuses it.
  revealedFolder: null as string | null,
  // The open page has unsaved edits or a save is running (toolbar dot).
  pageUnsaved: false,

  // UI state
  theme: 'light' as ThemeMode,
  editorTab: 'preview' as 'write' | 'preview',
  showSearch: false,
  // Text page search opens with; `>` opens it in command mode
  // (spec `command-palette`). Read when search opens.
  searchPrefill: '',
  // Counts openSearch calls. Search that is already showing (command mode
  // running "Search pages") takes the new prefill when this changes.
  searchOpens: 0,
  showExitPrompt: false,
  showNewPageForm: false,
  showVaultSelector: false,
  showHelp: false,
  // Open toolbar menu (page ▾ or ⋯). Shared so `closeAllDialogs` closes it and
  // global hotkeys treat it as a transient element (spec `application-lifecycle`).
  toolbarMenu: null as 'page' | 'more' | 'path' | null,
  renameUuid: null as string | null,
  renameTitle: '',
  confirmDeleteUuid: null as string | null,
  // Sidebar tree (spec `desktop-page-tree`): the page being moved, the folder
  // being renamed, and the folder a new page is created in.
  moveUuid: null as string | null,
  renameFolderPath: null as string | null,
  newPagePath: '',
  // Collapsed folders of the open vault; only collapsed ones are stored.
  collapsedFolders: [] as string[],
  // Collapsed folders of every vault, as loaded from ui.toml.
  uiCollapsedFolders: {} as Record<string, string[]>,
  // Views (spec `desktop-view-switcher`): the vault's views, the active one's
  // id (null is the default view "All notes"), its resolved tree and stored
  // nodes (what an edit sends back), a load error, and the collapsed folders of
  // the view (not remembered: a view's folders start expanded).
  views: [] as ViewMeta[],
  activeViewId: null as string | null,
  viewData: null as ViewData | null,
  viewNodes: [] as StoredNode[],
  viewError: null as string | null,
  viewCollapsed: [] as string[],
  // The chosen view of every vault, as loaded from ui.toml.
  uiActiveViews: {} as Record<string, string>,
  // The vault's sidebar layout from `.cerbo/tree.toml` (add-sidebar-ordering):
  // pinned folders and the order of each folder's children.
  layout: EMPTY_LAYOUT as TreeLayout,
  // New Folder dialog: open flag and the prefilled path.
  showNewFolder: false,
  newFolderPath: '',
  showImportVault: false,
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
    if (typeof res.leftSidebarVisible === 'boolean') {
      app.showLeftSidebar = res.leftSidebarVisible;
    }
    if (typeof res.lastSparqlQuery === 'string') {
      app.lastSparqlQuery = res.lastSparqlQuery;
    }
    app.rightPanelTab = parseRightPanel(res.rightPanelTab);
    app.uiCollapsedFolders = res.collapsedFolders ?? {};
    app.uiActiveViews = res.activeViews ?? {};
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
      leftSidebarVisible: app.showLeftSidebar,
      windowBounds: null, // kept: `app_exit` saves the window size
      lastSparqlQuery: app.lastSparqlQuery,
      rightPanelTab: app.rightPanelTab,
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
  app.searchPrefill = '';
  app.showExitPrompt = false;
  app.showNewPageForm = false;
  app.showVaultSelector = false;
  app.showHelp = false;
  app.toolbarMenu = null;
  app.renameUuid = null;
  app.renameTitle = '';
  app.confirmDeleteUuid = null;
  app.showSparqlConsole = false;
  app.moveUuid = null;
  app.renameFolderPath = null;
  app.newPagePath = '';
  app.showNewFolder = false;
  app.newFolderPath = '';
  app.showImportVault = false;
}

// ── Panels and editor mode ────────────────────────────────────────────────────

function panelState(): PanelState {
  return {
    widthClass: app.widthClass,
    showLeft: app.showLeftSidebar,
    showRight: app.showRightSidebar,
    overlay: app.overlay,
  };
}

function applyPanelState(next: { state: PanelState; persist: boolean }): void {
  app.showLeftSidebar = next.state.showLeft;
  app.showRightSidebar = next.state.showRight;
  app.overlay = next.state.overlay;
  if (next.persist) void saveUiSettings();
}

/** Whether `panel` is on screen in the current layout, docked or floating. */
export function isPanelShown(panel: Panel): boolean {
  return panelShown(panel, panelState());
}

/**
 * Use the page tree's toggle: a docked tree shows or hides and is remembered
 * across restarts; in the narrow layout the drawer opens or closes instead.
 */
export function toggleLeftSidebar(): void {
  applyPanelState(togglePanel('tree', panelState()));
}

/**
 * Use the right panel's toggle: docked in the wide layout (remembered), an
 * overlay in the medium and narrow layouts (not remembered).
 */
export function toggleRightSidebar(): void {
  applyPanelState(togglePanel('right', panelState()));
}

/** Select a right panel tab and remember it. The page does not matter: it stays across pages. */
export function selectRightPanelTab(id: RightPanelId): void {
  if (app.rightPanelTab === id) return;
  app.rightPanelTab = id;
  void saveUiSettings();
}

/**
 * A tab's hotkey or command (spec "Tab hotkeys and commands"): show the right
 * panel with the tab selected; select the tab in a shown panel; or hide the
 * panel when that tab is already the selected one.
 */
export function useRightPanelTab(id: RightPanelId): void {
  const outcome = tabShortcutOutcome(id, { panelShown: isPanelShown('right'), selected: app.rightPanelTab });
  if (outcome === 'hide') {
    toggleRightSidebar();
    return;
  }
  selectRightPanelTab(id);
  if (outcome === 'show') showPanelNow('right');
}

/** Make `panel` visible in the current layout, if it is not already. */
export function showPanelNow(panel: Panel): void {
  applyPanelState(showPanel(panel, panelState()));
}

/** Close the floating panel, if one is open (`Esc`, outside click). */
export function closeOverlay(): void {
  app.overlay = null;
}

/** Follow the window width; crossing a boundary closes any overlay. */
/**
 * The widths the docked tree and right panel are drawn at: the stored widths,
 * clamped so the editor keeps 360px in the current window (spec "Editor keeps
 * its width"). 0 for a panel that is not docked. Stored widths are untouched.
 */
export function dockedPanelWidths(): { tree: number; right: number } {
  const tree = app.showLeftSidebar && app.widthClass !== 'narrow' ? app.sidebarWidth : 0;
  const right = app.showRightSidebar && app.widthClass === 'wide' && app.currentUuid ? app.backlinksWidth : 0;
  return dockedWidths(app.windowWidth, tree, right);
}

export function setWindowWidth(width: number): void {
  app.windowWidth = width;
  const next = changeWidthClass(widthClassFor(width), panelState());
  if (next.widthClass !== app.widthClass) {
    app.widthClass = next.widthClass;
    app.overlay = next.overlay;
  }
}

// The page editor registers how it switches mode, so it can store the caret
// before the write view goes away. Without an editor the tab flips directly.
let editorModeToggle: (() => void) | null = null;

export function onEditorModeToggle(toggle: () => void): () => void {
  editorModeToggle = toggle;
  return () => {
    if (editorModeToggle === toggle) editorModeToggle = null;
  };
}

/** Switch the open page between edit and preview (`Ctrl+E`, toolbar toggle). */
export function toggleEditorMode(): void {
  if (!app.currentUuid) return;
  if (editorModeToggle) editorModeToggle();
  else app.editorTab = app.editorTab === 'write' ? 'preview' : 'write';
}

/** Open page search, optionally with text already typed (`>` = commands). */
export function openSearch(prefill = ''): void {
  closeAllDialogs();
  app.searchPrefill = prefill;
  app.searchOpens += 1;
  app.showSearch = true;
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

// Run before the open page is left without the editor seeing a page change
// first (vault switch, quit), so it can store where editing ended. The page
// editor registers one; a failing hook never blocks leaving.
const editorLeaveHooks = new Set<() => Promise<void> | void>();

export function onEditorLeave(hook: () => Promise<void> | void): () => void {
  editorLeaveHooks.add(hook);
  return () => editorLeaveHooks.delete(hook);
}

export async function runEditorLeaveHooks(): Promise<void> {
  await Promise.allSettled([...editorLeaveHooks].map(async (hook) => hook()));
}

export async function quitApp(): Promise<void> {
  try {
    await runEditorLeaveHooks();
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
  await runEditorLeaveHooks();
  app.loading = true;
  app.loadingMessage = 'Opening vault…';
  app.error = null;
  try {
    await invoke('vault_open', { vaultId: vaultId });
    app.activeVaultId = vaultId;
    await invoke('vault_set_active', { id: vaultId });
    await loadPages();
    app.collapsedFolders = pruneCollapsed(app.uiCollapsedFolders[vaultId] ?? [], app.pages, app.layout);
    await restoreActiveView(vaultId);
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
      app.outgoingLinks = [];
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
  // A layout that cannot be read leaves the tree in default order and says so.
  try {
    app.layout = await invoke<TreeLayout>('tree_layout');
  } catch (e) {
    app.layout = EMPTY_LAYOUT;
    setError(String(e));
  }
  // Titles and missing pages in the active view follow the pages.
  if (app.activeViewId) void loadActiveView();
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
    revealPage(uuid);
    await invoke('vault_update_last_page', { vaultId: app.activeVaultId, uuid });
    const v = activeVault();
    if (v) v.lastOpenPage = uuid;

    // Backlinks, outgoing links and similar notes belong to the Links tab,
    // which loads them itself while it is visible (spec `desktop-right-panel`,
    // design D3); loading them here too would load twice, and while hidden.
    await loadAttachments(uuid);
  } catch (e) {
    setError(String(e));
  }
}

// Next/previous follow the sidebar: visible page rows in tree order.
export async function openNextPage(): Promise<void> {
  const order = visiblePageUuids(visibleRows());
  if (order.length === 0) return;
  const currentIndex = order.indexOf(app.currentUuid ?? '');
  await openPage(order[(currentIndex + 1) % order.length]);
}

export async function openPrevPage(): Promise<void> {
  const order = visiblePageUuids(visibleRows());
  if (order.length === 0) return;
  const currentIndex = order.indexOf(app.currentUuid ?? '');
  const prevIndex = currentIndex < 0 ? order.length - 1 : (currentIndex - 1 + order.length) % order.length;
  await openPage(order[prevIndex]);
}

// ── Sidebar tree (spec `desktop-page-tree`, design D5, D6) ───────────────────

/** Whether the sidebar shows a view rather than the default tree. */
export function inView(): boolean {
  return app.activeViewId !== null && app.viewData !== null;
}

/** The sidebar tree: the active view's, or `app.pages` in the vault layout's order. */
export function pageTree(): TreeNode[] {
  if (inView() && app.viewData) return buildViewTree(app.viewData.nodes);
  return buildPageTree(app.pages, app.layout);
}

/** The collapsed folders of what the sidebar shows. */
export function collapsedPaths(): ReadonlySet<string> {
  return new Set(inView() ? app.viewCollapsed : app.collapsedFolders);
}

/** The sidebar rows: the tree minus collapsed subtrees. */
export function visibleRows(): TreeRow[] {
  return flattenVisible(pageTree(), collapsedPaths());
}

// ── Views (spec `desktop-view-switcher`) ─────────────────────────────────────

/** Reload the vault's view list (also when the dropdown opens: an agent may have added one). */
export async function loadViews(): Promise<void> {
  try {
    app.views = await invoke<ViewMeta[]>('view_list');
  } catch (e) {
    app.views = [];
    console.error('Failed to list views:', e);
  }
}

/** Load the active view's tree; an id that fails to load leaves the default tree and says why. */
export async function loadActiveView(): Promise<void> {
  const id = app.activeViewId;
  if (!id) {
    app.viewData = null;
    app.viewNodes = [];
    app.viewError = null;
    return;
  }
  try {
    const [data, nodes] = await Promise.all([
      invoke<ViewData>('view_get', { uuid: id }),
      invoke<StoredNode[]>('view_nodes', { uuid: id }),
    ]);
    if (app.activeViewId !== id) return;
    app.viewData = data;
    app.viewNodes = nodes;
    app.viewError = null;
  } catch (e) {
    if (app.activeViewId !== id) return;
    app.viewData = null;
    app.viewNodes = [];
    app.viewError = String(e);
  }
}

/** Restore the vault's remembered view; one that no longer exists is dropped. */
async function restoreActiveView(vaultId: string): Promise<void> {
  app.viewCollapsed = [];
  await loadViews();
  const saved = app.uiActiveViews[vaultId];
  app.activeViewId = saved && app.views.some((v) => v.uuid === saved) ? saved : null;
  if (saved && !app.activeViewId) {
    delete app.uiActiveViews[vaultId];
    invoke('ui_settings_set_active_view', { vaultId, viewId: null }).catch(() => {});
  }
  await loadActiveView();
}

/** Show view `id` (null: the default view) and remember the choice for this vault. */
export async function setActiveView(id: string | null): Promise<void> {
  const vaultId = app.activeVaultId;
  if (!vaultId || app.activeViewId === id) return;
  app.activeViewId = id;
  app.viewCollapsed = [];
  if (id) app.uiActiveViews[vaultId] = id;
  else delete app.uiActiveViews[vaultId];
  invoke('ui_settings_set_active_view', { vaultId, viewId: id }).catch((e) => {
    console.error('Failed to save the sidebar view:', e);
  });
  await loadActiveView();
}

async function saveViewNodes(id: string, nodes: StoredNode[]): Promise<void> {
  try {
    await invoke('view_update', { uuid: id, title: null, nodes });
  } catch (e) {
    setError(String(e));
    return;
  }
  if (app.activeViewId === id) await loadActiveView();
}

/** Take the node at `path` out of the active view; the page itself stays. */
export async function viewRemoveNode(path: number[]): Promise<void> {
  const id = app.activeViewId;
  if (!id || !inView()) return;
  await saveViewNodes(id, removeAt(app.viewNodes, path));
}

/** Move the node at `path` one place among its siblings in the active view. */
export async function viewMoveNode(path: number[], delta: -1 | 1): Promise<void> {
  const id = app.activeViewId;
  if (!id || !inView()) return;
  const next = moveAt(app.viewNodes, path, delta);
  if (next !== app.viewNodes) await saveViewNodes(id, next);
}

/** Add a reference to page `uuid` at the end of view `viewId`, whether or not it is the active one. */
export async function addPageToView(viewId: string, uuid: string): Promise<void> {
  try {
    const stored = viewId === app.activeViewId && inView()
      ? app.viewNodes
      : await invoke<StoredNode[]>('view_nodes', { uuid: viewId });
    await invoke('view_update', { uuid: viewId, title: null, nodes: appendPage(stored, uuid) });
  } catch (e) {
    setError(String(e));
    return;
  }
  if (app.activeViewId === viewId) await loadActiveView();
}

/** Set the collapsed folders of the open vault and remember them in ui.toml. */
export function setCollapsedFolders(paths: string[]): void {
  const vaultId = app.activeVaultId;
  if (!vaultId) return;
  const next = pruneCollapsed(paths, app.pages, app.layout);
  const unchanged = next.length === app.collapsedFolders.length && next.every((p) => app.collapsedFolders.includes(p));
  app.collapsedFolders = next;
  if (unchanged) return;
  app.uiCollapsedFolders[vaultId] = next;
  invoke('ui_settings_set_collapsed_folders', { vaultId, paths: next }).catch((e) => {
    console.error('Failed to save collapsed folders:', e);
  });
}

export function collapseFolder(path: string): void {
  if (inView()) {
    if (!app.viewCollapsed.includes(path)) app.viewCollapsed = [...app.viewCollapsed, path];
    return;
  }
  if (!app.collapsedFolders.includes(path)) setCollapsedFolders([...app.collapsedFolders, path]);
}

export function expandFolder(path: string): void {
  if (inView()) {
    app.viewCollapsed = app.viewCollapsed.filter((p) => p !== path);
    return;
  }
  if (app.collapsedFolders.includes(path)) setCollapsedFolders(app.collapsedFolders.filter((p) => p !== path));
}

export function toggleFolder(path: string): void {
  if (collapsedPaths().has(path)) expandFolder(path);
  else collapseFolder(path);
}

/** Expand every folder on the page's path so its row is visible. */
export function revealPage(uuid: string): void {
  const page = app.pages.find((p) => p.uuid === uuid);
  if (!page) return;
  const next = revealPath(app.collapsedFolders, page.virtualPath);
  if (next.length !== app.collapsedFolders.length) setCollapsedFolders(next);
}

/**
 * Show the page tree and select the folder `path`, expanding its parents
 * (spec `desktop-window-toolbar`, "Folder segment reveals the folder").
 */
export function revealFolder(path: string): void {
  showPanelNow('tree');
  const parent = ancestorFolders(path).slice(0, -1).at(-1) ?? null;
  const next = revealPath(app.collapsedFolders, parent);
  if (next.length !== app.collapsedFolders.length) setCollapsedFolders(next);
  // Reassign through null so picking the same folder again refocuses it.
  app.revealedFolder = null;
  app.revealedFolder = path;
}

export interface FolderRenameReport {
  changed: string[];
  failed: [string, string][];
}

/** Move a page to `path` (null for the root) and show it in its new folder. */
export async function movePage(uuid: string, path: string | null): Promise<void> {
  if (!app.activeVaultId) throw new Error('No active vault');
  // The reload is queued with the write, so two quick moves cannot leave
  // `app.pages` showing the older listing.
  await vaultWrite(async () => {
    await invoke('page_move', { uuid, virtualPath: path });
    await loadPages();
  });
  setCollapsedFolders(revealPath(app.collapsedFolders, path));
}

/**
 * Rename the folder `from` to `to` on every page under it. Collapsed state
 * follows the folder. Pages that could not be moved are reported in the error
 * banner; the rest are moved.
 */
export async function renameFolder(from: string, to: string): Promise<FolderRenameReport> {
  if (!app.activeVaultId) throw new Error('No active vault');
  const report = await vaultWrite(async () => {
    const r = await invoke<FolderRenameReport>('folder_rename', { oldPrefix: from, newPrefix: to });
    await loadPages();
    return r;
  });
  setCollapsedFolders(carryCollapsed(app.collapsedFolders, from, to));
  if (report.failed.length > 0) {
    const details = report.failed.map(([uuid, err]) => `${uuid}: ${err}`).join('; ');
    setError(`${report.failed.length} pages could not be moved: ${details}`);
  }
  return report;
}

/**
 * Apply a placement (add-sidebar-ordering D6): move the page or folder when it
 * changes folder, then store the parent's new order. One reload at the end.
 */
export async function placeRow(p: Placement): Promise<void> {
  if (!app.activeVaultId) throw new Error('No active vault');
  let failed: [string, string][] = [];
  try {
    await vaultWrite(async () => {
      try {
        if (p.move?.kind === 'page') {
          await invoke('page_move', { uuid: p.move.uuid, virtualPath: p.move.to });
        } else if (p.move?.kind === 'folder') {
          const r = await invoke<FolderRenameReport>('folder_rename', { oldPrefix: p.move.from, newPrefix: p.move.to });
          failed = r.failed;
        }
        await invoke('folder_set_order', { parent: p.parent, entries: p.order });
      } finally {
        await loadPages();
      }
    });
  } finally {
    if (p.move?.kind === 'folder') setCollapsedFolders(carryCollapsed(app.collapsedFolders, p.move.from, p.move.to));
  }
  // The row lands in `parent`, which must be open to show it.
  setCollapsedFolders(revealPath(app.collapsedFolders, p.parent || null));
  if (failed.length > 0) {
    const details = failed.map(([uuid, err]) => `${uuid}: ${err}`).join('; ');
    setError(`${failed.length} pages could not be moved: ${details}`);
  }
}

/** Create an empty (pinned) folder and show it. */
export async function createFolder(path: string): Promise<void> {
  if (!app.activeVaultId) throw new Error('No active vault');
  await vaultWrite(async () => {
    await invoke('folder_create', { path });
    await loadPages();
  });
  setCollapsedFolders(revealPath(app.collapsedFolders, ancestorFolders(path).slice(0, -1).at(-1) ?? null));
}

/** Delete a folder that has no pages under it. */
export async function deleteFolder(path: string): Promise<void> {
  if (!app.activeVaultId) throw new Error('No active vault');
  await vaultWrite(async () => {
    await invoke('folder_delete', { path });
    await loadPages();
  });
}

export function triggerNewFolder(parent = '') {
  closeAllDialogs();
  app.newFolderPath = parent ? `${parent}/` : '';
  app.showNewFolder = true;
}

export function triggerImportVault() {
  closeAllDialogs();
  app.showImportVault = true;
}

export function triggerMove(uuid?: string) {
  const targetUuid = uuid || app.currentUuid;
  if (!targetUuid || !app.pages.some((p) => p.uuid === targetUuid)) return;
  closeAllDialogs();
  app.moveUuid = targetUuid;
}

export function triggerRenameFolder(path: string) {
  closeAllDialogs();
  app.renameFolderPath = path;
}

export function triggerNewPageIn(path: string) {
  closeAllDialogs();
  app.newPagePath = path;
  app.showNewPageForm = true;
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
        app.outgoingLinks = [];
      }
    }
  } catch (e) {
    setError(String(e));
    throw e;
  }
}

// `reopen: false` is for a page the editor already left: its title is updated
// without navigating back to it.
export async function updatePageTitle(uuid: string, newTitle: string, options?: { reopen?: boolean }): Promise<void> {
  if (!app.activeVaultId) throw new Error('No active vault');
  await vaultWrite(() => invoke('page_update_title', { uuid, newTitle }));
  await loadPages();
  if (options?.reopen ?? true) await openPage(uuid);
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
 * Load the Links tab's lists (backlinks and outgoing links) of page `uuid`,
 * but only while the tab is visible: a hidden tab loads nothing (design D3).
 */
export function loadLinksWhenVisible(uuid: string | null | undefined, visible: boolean): void {
  if (!uuid || !visible) return;
  void loadBacklinks(uuid);
  void loadOutgoingLinks(uuid);
}

/** `loadSimilarNotes`, but only while the Links tab is visible. */
export function loadSimilarNotesWhenVisible(uuid: string | null | undefined, visible: boolean): void {
  if (!uuid || !visible) return;
  void loadSimilarNotes(uuid);
}

let outgoingRequest = 0;

/** Load the pages that page `uuid` links to; answers for a page left behind are dropped. */
export async function loadOutgoingLinks(uuid: string): Promise<void> {
  if (!app.activeVaultId) return;
  const request = ++outgoingRequest;
  app.outgoingStatus = 'loading';
  app.outgoingMessage = '';
  try {
    const entries = await invoke<BacklinkEntry[]>('outgoing_links_get', {
      vaultId: app.activeVaultId,
      uuid,
    });
    if (request !== outgoingRequest) return;
    app.outgoingLinks = entries;
    app.outgoingStatus = 'ready';
  } catch (e) {
    if (request !== outgoingRequest) return;
    const failure = backlinksFailure(e, 'Outgoing links');
    app.outgoingLinks = [];
    app.outgoingStatus = failure.status;
    app.outgoingMessage = failure.message;
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

/** One attachment a page links to, as `attachment_list` returns it. */
export interface AttachmentInfo {
  /** The attachment object's own UUID. */
  uuid: string;
  /** The file's original name. */
  title: string;
  mime_type: string | null;
}

// Loads overlap (opening a page, the panel becoming visible, a save that added
// a reference): only the newest request may write, or a slow older answer
// would replace a newer list that already holds the new Attachment.
let attachmentsRequest = 0;

export async function loadAttachments(uuid: string): Promise<void> {
  if (!app.activeVaultId || !uuid) return;
  const request = ++attachmentsRequest;
  try {
    const attachments = await invoke<AttachmentInfo[]>('attachment_list', {
      vaultId: app.activeVaultId,
      uuid,
    });
    if (request === attachmentsRequest) app.attachments = attachments;
  } catch (e) {
    console.error('Failed to load attachments:', e);
    if (request === attachmentsRequest) app.attachments = [];
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
