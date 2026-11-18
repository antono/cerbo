<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import { fade } from 'svelte/transition';
  import { ModeWatcher, mode, setMode } from 'mode-watcher';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { Plus } from 'lucide-svelte';
  import cerboIcon from '../favicon.svg';
  import { app, loadVaults, loadUiSettings, saveUiSettings, openVault, quickAddVault, closeAllDialogs, closeOverlay, setWindowWidth, dockedPanelWidths } from '#lib/stores.svelte.js';
  import PageList from '#lib/PageList.svelte';
  import GlobalSearch from '#lib/GlobalSearch.svelte';
  import NewPageDialog from '#lib/NewPageDialog.svelte';
  import RenamePageDialog from '#lib/RenamePageDialog.svelte';
  import MovePageDialog from '#lib/MovePageDialog.svelte';
  import RenameFolderDialog from '#lib/RenameFolderDialog.svelte';
  import NewFolderDialog from '#lib/NewFolderDialog.svelte';
  import KeyboardHelp from '#lib/KeyboardHelp.svelte';
  import SparqlConsole from '#lib/SparqlConsole.svelte';
  import ExitConfirmation from '#lib/ExitConfirmation.svelte';
  import DeletePageDialog from '#lib/DeletePageDialog.svelte';
  import WindowToolbar from '#lib/WindowToolbar.svelte';
  import PanelOverlay from '#lib/PanelOverlay.svelte';
  import { agentStore, initAgentStore } from '#lib/agent/store.svelte.js';
  import { maxPanelWidth } from '#lib/layout.js';
  import { splitVaultPath } from '#lib/vault-path.js';
  import { isInputFocused, confirmationOpen, anyDialogOpen as transientOpen } from '#lib/hotkeys.js';
  import { dispatchHotkey, pressedKey } from '#lib/actions.js';
  import { appActions } from '#lib/action-host.js';
  import '../app.css';

  let { children } = $props();

  // Focusable panel refs
  let sidebarEl = $state<HTMLElement | null>(null);
  let mainEl = $state<HTMLElement | null>(null);
  let vaultSelectorEl = $state<HTMLElement | null>(null);
  let selectedVaultIndex = $state(0);

  /** Any dialog, search, the vault selector or a toolbar menu is open (confirmations included). */
  function anyDialogOpen(): boolean {
    return transientOpen(app);
  }

  // Width class from one ResizeObserver (design D4): the layout follows the
  // window while it is being resized, without a reload.
  // It watches <html>, not <body>: the body draws a 1px window border, so a
  // 1000px window would measure 998px and fall into the medium class.
  onMount(() => {
    setWindowWidth(document.documentElement.clientWidth || window.innerWidth);
    const observer = new ResizeObserver((entries) => {
      const width = entries[0]?.contentRect.width;
      if (width) setWindowWidth(width);
    });
    observer.observe(document.documentElement);
    return () => observer.disconnect();
  });

  // Overlays close on Esc (spec `desktop-responsive-layout`, "Overlays close
  // predictably"). Captured before the editor sees it, so Esc closes the
  // drawer or panel and does not also leave edit mode.
  $effect(() => {
    function closeOverlayOnEscape(e: KeyboardEvent) {
      if (e.key !== 'Escape' || app.overlay === null || anyDialogOpen()) return;
      // Esc closes the innermost element first: a tree row menu or a tree
      // drag inside the drawer handles it (its own capture listener) and the
      // drawer stays open.
      if (document.querySelector('.row-menu') || document.body.classList.contains('tree-dragging')) return;
      e.preventDefault();
      e.stopPropagation();
      closeOverlay();
    }
    window.addEventListener('keydown', closeOverlayOnEscape, true);
    return () => window.removeEventListener('keydown', closeOverlayOnEscape, true);
  });

  // Picking a page in the narrow tree drawer closes the drawer.
  let lastUuid: string | null = null;
  $effect(() => {
    const uuid = app.currentUuid;
    untrack(() => {
      if (uuid !== lastUuid && app.overlay === 'tree') closeOverlay();
      lastUuid = uuid;
    });
  });

  // The agent store outlives the Agent tab: an approval that arrives while the
  // panel is hidden still marks the tab. It follows the open vault.
  onMount(() => {
    initAgentStore();
    return () => agentStore.dispose();
  });
  $effect(() => {
    const vault = app.activeVaultId;
    untrack(() => void agentStore.setVault(vault));
  });

  onMount(async () => {
    await loadUiSettings();
    setMode(app.theme as 'light' | 'dark' | 'system');
    await loadVaults();
    if (app.activeVaultId) {
      await openVault(app.activeVaultId);
    }
  });

  // Global keydown listener. App hotkeys come from the action registry
  // (`actions.ts`); only Escape and preview scrolling are handled here.
  $effect(() => {
    function handleKeydown(e: KeyboardEvent) {
      // 1. Escape: Close active dialogs/forms
      if (e.key === 'Escape') {
        if (app.showSearch || app.showNewPageForm || app.showVaultSelector || app.showHelp || app.renameUuid || app.confirmDeleteUuid || app.showSparqlConsole || app.moveUuid || app.renameFolderPath || app.showNewFolder) {
          closeAllDialogs();
          return;
        }
      }

      // Confirmation dialogs own their own keyboard handling.
      if (confirmationOpen(app)) return;

      const inputFocused = isInputFocused();
      if (dispatchHotkey(e, { dialogOpen: anyDialogOpen(), inputFocused, editorTab: app.editorTab }, appActions)) return;

      if (anyDialogOpen()) return;

      // Preview scrolling, vim-style: j/ArrowDown down, k/ArrowUp up.
      // `pressedKey` reads the physical key on a Cyrillic layout (`о` -> j).
      if (app.editorTab === 'preview' && !inputFocused && !e.ctrlKey && !e.metaKey && !e.altKey) {
        const k = pressedKey(e);
        if ((k === 'j' && !e.shiftKey) || e.key === 'ArrowDown') {
          e.preventDefault();
          const renderer = document.querySelector('.carta-renderer') as HTMLElement;
          if (renderer) renderer.scrollBy({ top: 100, behavior: 'smooth' });
          return;
        }
        if ((k === 'k' && !e.shiftKey) || e.key === 'ArrowUp') {
          e.preventDefault();
          const renderer = document.querySelector('.carta-renderer') as HTMLElement;
          if (renderer) renderer.scrollBy({ top: -100, behavior: 'smooth' });
          return;
        }
      }
    }

    window.addEventListener('keydown', handleKeydown);
    return () => window.removeEventListener('keydown', handleKeydown);
  });

  $effect(() => {
    // Synchronize Tauri window theme with app theme
    if (mode.current) {
      app.theme = mode.current;
      const win = getCurrentWindow();
      win.setTheme(mode.current === 'dark' ? 'dark' : 'light').catch(() => {
        // Ignore errors if not running in Tauri or permission denied
      });
    }
  });

  let vault = $derived(app.vaults.find((v) => v.id === app.activeVaultId));
  // Drawn width of the docked tree: re-clamped whenever the window resizes.
  let treeWidth = $derived(dockedPanelWidths().tree);

  $effect(() => {
    if (!app.showVaultSelector) return;
    selectedVaultIndex = Math.max(0, app.vaults.findIndex((v) => v.id === app.activeVaultId));
    vaultSelectorEl?.focus();
  });

  // ── Resizing Logic ──────────────────────────────────────────────────────────

  let isResizingSidebar = $state(false);

  function startSidebarResize() {
    isResizingSidebar = true;
    window.addEventListener('mousemove', handleSidebarResize);
    window.addEventListener('mouseup', stopResizing);
  }

  function handleSidebarResize(e: MouseEvent) {
    if (!isResizingSidebar) return;
    // Enforce min/max constraints
    // The editor keeps at least 360px (spec "Editor keeps its width").
    const right = dockedPanelWidths().right;
    app.sidebarWidth = Math.max(200, Math.min(maxPanelWidth(window.innerWidth, right, 200, 600), e.clientX));
  }

  function stopResizing() {
    isResizingSidebar = false;
    window.removeEventListener('mousemove', handleSidebarResize);
    window.removeEventListener('mouseup', stopResizing);
    saveUiSettings();
  }

  function handleVaultSelectorKeydown(e: KeyboardEvent) {
    e.stopPropagation();

    if (e.key === 'Escape') {
      e.preventDefault();
      app.showVaultSelector = false;
      return;
    }

    if (!app.vaults.length) {
      if (e.key === 'Enter') {
        e.preventDefault();
        app.showVaultSelector = false;
        void quickAddVault();
      }
      return;
    }

    if (e.key === 'ArrowDown' || e.key === 'j') {
      e.preventDefault();
      selectedVaultIndex = (selectedVaultIndex + 1) % app.vaults.length;
      return;
    }

    if (e.key === 'ArrowUp' || e.key === 'k') {
      e.preventDefault();
      selectedVaultIndex = (selectedVaultIndex - 1 + app.vaults.length) % app.vaults.length;
      return;
    }

    if (e.key === 'Home') {
      e.preventDefault();
      selectedVaultIndex = 0;
      return;
    }

    if (e.key === 'End') {
      e.preventDefault();
      selectedVaultIndex = app.vaults.length - 1;
      return;
    }

    if (e.key === 'Enter') {
      e.preventDefault();
      const selected = app.vaults[selectedVaultIndex];
      if (selected) {
        void openVault(selected.id).then(() => {
          app.showVaultSelector = false;
        });
      }
    }
  }

  function selectVault(vaultId: string) {
    void openVault(vaultId).then(() => {
      app.showVaultSelector = false;
    });
  }
</script>

<ModeWatcher />

<WindowToolbar />

<div class="app-shell" class:is-resizing={isResizingSidebar}>
  <!-- Sidebar: hidden with Ctrl+\ (spec `keyboard-shortcuts`). Docked in the
       wide and medium layouts; a drawer (PanelOverlay) in the narrow one. -->
  {#if app.showLeftSidebar && app.widthClass !== 'narrow'}
  <aside 
    class="sidebar" 
    style="width: {treeWidth}px;"
    bind:this={sidebarEl}
    tabindex="-1"
  >
    <!-- Page list -->
    <div class="page-list-wrap">
      {#if vault}
        <PageList />
      {:else}
        <div class="no-vault">
          <p>Add a vault to get started.</p>
          <button class="primary-btn" onclick={() => void quickAddVault()}>Add Vault</button>
        </div>
      {/if}
    </div>

  </aside>

  <!-- Resize Handle -->
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div 
    class="resize-handle" 
    onmousedown={startSidebarResize}
    role="separator"
    aria-orientation="vertical"
  ></div>
  {/if}

  <!-- Main content -->
  <main 
    class="main-area"
    bind:this={mainEl}
    tabindex="-1"
  >
    {#if app.loading}
      <div class="loading-overlay">
        <div class="loading-spinner"></div>
        <p>{app.loadingMessage || 'Loading…'}</p>
      </div>
    {:else}
      {@render children()}
    {/if}
  </main>

  <PanelOverlay />
</div>

<!-- ── Modals ──────────────────────────────────────────────────────────────── -->

{#if app.showSearch}
  <GlobalSearch onClose={() => app.showSearch = false} />
{/if}

{#if app.showNewPageForm}
  <NewPageDialog onClose={() => { app.showNewPageForm = false; app.newPagePath = ''; }} />
{/if}

{#if app.renameUuid}
  <RenamePageDialog onClose={() => app.renameUuid = null} />
{/if}

<!-- Rendered here, not in the tree, so it shows whatever the layout or tree
     visibility (narrow page menu, Del hotkey with the tree hidden). -->
{#if app.confirmDeleteUuid}
  <DeletePageDialog />
{/if}

{#if app.moveUuid}
  <MovePageDialog onClose={() => app.moveUuid = null} />
{/if}

{#if app.renameFolderPath !== null}
  <RenameFolderDialog onClose={() => app.renameFolderPath = null} />
{/if}

{#if app.showNewFolder}
  <NewFolderDialog onClose={() => { app.showNewFolder = false; app.newFolderPath = ''; }} />
{/if}

{#if app.showHelp}
  <KeyboardHelp onClose={() => app.showHelp = false} />
{/if}

{#if app.showSparqlConsole}
  <SparqlConsole onClose={() => app.showSparqlConsole = false} />
{/if}

{#if app.showVaultSelector}
  <div class="modal-backdrop" onclick={() => (app.showVaultSelector = false)} role="presentation">
    <div bind:this={vaultSelectorEl} class="vault-selector-modal" onkeydown={handleVaultSelectorKeydown} onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true" tabindex="-1" transition:fade={{ duration: 150 }}>
      <header class="modal-header">
        <h2>Switch Vault</h2>
        <button class="close-btn" onclick={() => (app.showVaultSelector = false)} title="Close (Esc)" aria-label="Close"><kbd class="close-kbd">Esc</kbd></button>
      </header>

      <div class="vault-selector-body">
        {#each app.vaults as item (item.id)}
          {@const path = splitVaultPath(item.path)}
          <button class="vault-item" class:selected={item.id === app.vaults[selectedVaultIndex]?.id} onclick={() => selectVault(item.id)} title={item.path}>
            <img src={cerboIcon} alt="" width="18" height="18" class="vault-icon" />
            <span class="vault-item-name">{item.name}</span>
            <!-- Middle truncation: the head shrinks with an ellipsis, the tail
                 (vault folder name) stays visible. -->
            <span class="vault-item-path" data-path={item.path}><span class="vault-item-path-head">{path.head}</span><span class="vault-item-path-tail">{path.tail}</span></span>
            {#if item.id === app.activeVaultId}
              <span class="vault-item-check">✓</span>
            {/if}
          </button>
        {/each}

        {#if app.vaults.length === 0}
          <p class="empty-hint">No vaults yet.</p>
        {/if}
      </div>

      <footer class="vault-selector-footer">
        <button class="add-vault-btn" onclick={async () => { app.showVaultSelector = false; await quickAddVault(); }} disabled={app.loading}>
          <Plus size={16} />
          <span>{app.loading ? 'Adding…' : 'Add vault'}</span>
        </button>
      </footer>
    </div>
  </div>
{/if}

{#if app.showExitPrompt}
  <ExitConfirmation onClose={() => app.showExitPrompt = false} />
{/if}

<!-- ── Error toast ─────────────────────────────────────────────────────────── -->
{#if app.error}
  <div class="error-toast" role="alert">
    <span>{app.error}</span>
    <button class="toast-close" onclick={() => (app.error = null)}>Dismiss</button>
  </div>
{/if}

<style>
  .app-shell {
    position: relative;
    display: flex;
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .sidebar {
    background: var(--sidebar-bg);
    border-right: none; /* Resize handle handles the divider */
    display: flex;
    flex-direction: column;
    overflow: hidden;
    height: 100%;
  }

  .resize-handle {
    width: 1px;
    background: var(--border);
    cursor: col-resize;
    transition: background 0.15s, width 0.15s;
    position: relative;
    z-index: 40;
    flex-shrink: 0;
  }

  .resize-handle:hover, .is-resizing .resize-handle {
    background: var(--primary);
    width: 2px;
  }

  /* Transparent wider target area for the resize handle */
  .resize-handle::after {
    content: '';
    position: absolute;
    top: 0;
    bottom: 0;
    left: -4px;
    right: -4px;
  }

  .is-resizing {
    cursor: col-resize;
    user-select: none;
  }

  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,0.4);
    backdrop-filter: blur(2px);
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 20vh;
    z-index: 1000;
  }

  .vault-selector-modal {
    width: 100%;
    max-width: 600px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg, 12px);
    box-shadow: 0 10px 40px rgba(0, 0, 0, 0.2);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 1rem 1.25rem;
    border-bottom: 1px solid var(--border);
  }

  .modal-header h2 {
    margin: 0;
    font-size: 1.1rem;
    font-weight: 600;
  }

  .close-btn {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--muted-foreground);
    padding: 0.25rem;
    border-radius: var(--radius);
  }

  .close-btn:hover {
    background: var(--accent-hover);
    color: var(--fg);
  }

  /* "Esc" instead of a ✕: the toolbar's quit button is the only ✕ in the
     window (spec `desktop-window-chrome`, "Close button inside the app"). */
  .close-kbd {
    font-family: inherit;
    font-size: 0.7rem;
    padding: 0.1rem 0.35rem;
    border: 1px solid var(--border);
    border-radius: 0.25rem;
  }

  .vault-selector-body {
    padding: 0.5rem;
    max-height: 50vh;
    overflow-y: auto;
  }

  /* Cerbo app icon (src/favicon.svg) as the vault glyph; it carries its own
     dark rounded background, so it reads on both themes. */
  .vault-icon {
    flex-shrink: 0;
    display: block;
  }

  .vault-item {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    width: 100%;
    padding: 0.6rem 0.75rem;
    border: none;
    background: transparent;
    border-radius: var(--radius);
    cursor: pointer;
    text-align: left;
    color: var(--fg);
    transition: all 0.1s;
  }

  .vault-item:hover {
    background: var(--accent-hover);
  }

  .vault-item.selected {
    background: var(--highlight);
    color: var(--highlight-fg);
  }

  .vault-item-check {
    color: inherit;
    flex-shrink: 0;
  }

  .vault-item-name {
    flex: 0 1 auto;
    max-width: 50%;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .vault-item-path {
    flex: 1 1 0;
    display: flex;
    min-width: 0;
    font-size: 0.8rem;
    color: var(--muted-foreground);
    white-space: nowrap;
  }

  .vault-item.selected .vault-item-path {
    color: inherit;
    opacity: 0.75;
  }

  .vault-item-path-head {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .vault-item-path-tail {
    flex-shrink: 0;
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .vault-selector-footer {
    padding: 0.75rem;
    border-top: 1px solid var(--border);
    background: var(--sidebar-bg);
  }

  .add-vault-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    width: 100%;
    padding: 0.5rem 1rem;
    background: var(--primary);
    color: #fff;
    border: none;
    border-radius: var(--radius);
    cursor: pointer;
    font-size: 0.8125rem;
    font-weight: 600;
    white-space: nowrap;
    transition: background 0.15s, transform 0.1s;
    box-shadow: 0 1px 2px rgba(0,0,0,0.1);
  }

  .add-vault-btn:hover {
    background: var(--primary-hover);
  }

  .add-vault-btn:active {
    transform: translateY(1px);
  }

  .page-list-wrap {
    flex: 1;
    overflow-y: auto;
    min-height: 0;
  }

  .no-vault {
    padding: 1.5rem 1rem;
    text-align: center;
    color: var(--muted-foreground);
    font-size: 0.875rem;
  }

  .no-vault p {
    margin: 0 0 0.75rem;
  }

  .no-vault .primary-btn {
    padding: 0.5rem 1.25rem;
    background: var(--primary);
    color: #fff;
    border: none;
    border-radius: var(--radius);
    cursor: pointer;
    font-size: 0.875rem;
    font-weight: 600;
    transition: background 0.15s, transform 0.1s;
    box-shadow: 0 1px 2px rgba(0,0,0,0.05);
  }

  .no-vault .primary-btn:hover {
    background: var(--primary-hover);
  }

  .no-vault .primary-btn:active {
    transform: translateY(1px);
  }

  .main-area {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: var(--bg);
  }

  .loading-overlay {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 1rem;
    color: var(--muted-foreground);
  }

  .loading-spinner {
    width: 2rem;
    height: 2rem;
    border: 3px solid var(--border);
    border-top-color: var(--primary);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .error-toast {
    position: fixed;
    bottom: 1.5rem;
    right: 1.5rem;
    background: #ef4444;
    color: #fff;
    padding: 0.6rem 1rem;
    border-radius: var(--radius);
    display: flex;
    align-items: center;
    gap: 0.75rem;
    box-shadow: 0 4px 12px rgba(0,0,0,0.2);
    font-size: 0.875rem;
    z-index: 200;
    max-width: 28rem;
  }

  .toast-close {
    background: none;
    border: none;
    color: #fff;
    cursor: pointer;
    font-size: 0.75rem;
    line-height: 1;
    padding: 0;
    flex-shrink: 0;
    text-decoration: underline;
  }
</style>
