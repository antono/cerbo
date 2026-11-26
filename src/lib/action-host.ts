/**
 * Binds the action registry (`actions.ts`) to the app store. Every consumer of
 * the registry (hotkeys, toolbar menus, command mode) runs actions with this
 * context, so a menu entry does exactly what its hotkey does.
 */

import { mode, setMode } from 'mode-watcher';
import type { ActionContext } from './actions';
import { insertionFor } from './link-picker';
import { clampZoom } from './zoom';
import { applyZoom } from './zoom-host';
import {
  app,
  closeAllDialogs,
  goBack,
  goForward,
  isPanelShown,
  openNextPage,
  openPrevPage,
  openSearch,
  quickAddVault,
  triggerImportVault,
  revealPage,
  saveUiSettings,
  setError,
  showPanelNow,
  toggleEditorMode,
  toggleLeftSidebar,
  toggleRightSidebar,
  triggerDelete,
  triggerMove,
  triggerNewFolder,
  triggerRename,
  useRightPanelTab,
} from './stores.svelte';

const DIALOG_FLAGS = {
  search: 'showSearch',
  newPage: 'showNewPageForm',
  vaultSelector: 'showVaultSelector',
  help: 'showHelp',
  sparqlConsole: 'showSparqlConsole',
} as const;

/** The live app as an `ActionContext`: state getters read the store on each access. */
export const appActions: ActionContext = {
  get hasVault() {
    return app.vaults.some((v) => v.id === app.activeVaultId);
  },
  get currentUuid() {
    return app.currentUuid;
  },
  get theme() {
    return mode.current === 'dark' ? 'dark' : 'light';
  },
  get editorTab() {
    return app.editorTab;
  },
  // Shown in the current layout, docked or as an overlay
  // (spec `desktop-responsive-layout`), so labels and pressed states match the screen.
  get showLeftSidebar() {
    return isPanelShown('tree');
  },
  get showRightSidebar() {
    return isPanelShown('right');
  },
  get rightPanelTab() {
    return app.rightPanelTab;
  },
  get canGoBack() {
    return app.historyIndex > 0;
  },
  get canGoForward() {
    return app.historyIndex < app.history.length - 1;
  },
  get zoom() {
    return app.zoom;
  },

  toggleDialog(dialog) {
    const flag = DIALOG_FLAGS[dialog];
    const next = !app[flag];
    closeAllDialogs();
    app[flag] = next;
  },
  openSearch(prefill) {
    openSearch(prefill);
  },
  toggleSearch(prefill) {
    if (app.showSearch) closeAllDialogs();
    else openSearch(prefill);
  },
  addVault() {
    closeAllDialogs();
    void quickAddVault();
  },
  importVault() {
    triggerImportVault();
  },
  toggleLeftSidebar,
  toggleRightSidebar,
  useRightPanelTab,
  toggleEditorMode,
  setTheme(theme) {
    app.theme = theme;
    setMode(theme);
    void saveUiSettings();
  },
  goBack() {
    void goBack();
  },
  goForward() {
    void goForward();
  },
  nextPage() {
    void openNextPage();
  },
  prevPage() {
    void openPrevPage();
  },
  renamePage() {
    triggerRename();
  },
  movePage() {
    triggerMove();
  },
  deletePage() {
    triggerDelete();
  },
  copyPageLink() {
    const page = app.pages.find((p) => p.uuid === app.currentUuid);
    if (!page) return;
    // The same Markdown link the `[` link picker inserts.
    navigator.clipboard.writeText(insertionFor('link', page)).catch((e) => setError(`Copy link failed: ${e}`));
  },
  revealPage() {
    if (!app.currentUuid) return;
    showPanelNow('tree');
    revealPage(app.currentUuid);
  },
  newFolder() {
    triggerNewFolder();
  },
  setZoom(zoom) {
    const next = clampZoom(zoom);
    if (next === app.zoom) return;
    app.zoom = next;
    void applyZoom(next);
    void saveUiSettings();
  },
  quit() {
    closeAllDialogs();
    app.showExitPrompt = true;
  },
};
