<script lang="ts">
  import { onMount } from 'svelte';
  import { fade } from 'svelte/transition';
  import { ModeWatcher, mode, setMode } from 'mode-watcher';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { ChevronDown, HelpCircle, Moon, Plus, Sun, Vault } from 'lucide-svelte';
  import { app, loadVaults, loadUiSettings, saveUiSettings, openVault, quickAddVault, quitApp, closeAllDialogs, openNextPage, openPrevPage, triggerRename, triggerDelete, goBack, goForward } from '$lib/stores.svelte';
  import PageList from '$lib/PageList.svelte';
  import GlobalSearch from '$lib/GlobalSearch.svelte';
  import NewPageDialog from '$lib/NewPageDialog.svelte';
  import RenamePageDialog from '$lib/RenamePageDialog.svelte';
  import KeyboardHelp from '$lib/KeyboardHelp.svelte';
  import ExitConfirmation from '$lib/ExitConfirmation.svelte';
  import { isModKey, isInputFocused } from '$lib/hotkeys';
  import '../app.css';

  let { children } = $props();

  // Focusable panel refs
  let sidebarEl = $state<HTMLElement | null>(null);
  let mainEl = $state<HTMLElement | null>(null);
  let vaultSelectorEl = $state<HTMLElement | null>(null);
  let selectedVaultIndex = $state(0);

  const modKeyLabel = typeof navigator !== 'undefined' && /Mac|iPod|iPhone|iPad/.test(navigator.userAgent) ? '⌘' : 'Ctrl';

  onMount(async () => {
    await loadUiSettings();
    setMode(app.theme as 'light' | 'dark' | 'system');
    await loadVaults();
    if (app.activeVaultId) {
      await openVault(app.activeVaultId);
    }
  });

  // Global keydown listener
  $effect(() => {
    function handleKeydown(e: KeyboardEvent) {
      // 1. Escape: Close active dialogs/forms
      if (e.key === 'Escape') {
        if (app.showSearch || app.showNewPageForm || app.showVaultSelector || app.showHelp || app.renameSlug || app.confirmDeleteSlug) {
          closeAllDialogs();
          return;
        }
      }

      // 2. New Page (Ctrl+N) - Handle first so it can toggle itself
      if (isModKey(e, 'n')) {
        e.preventDefault();
        const nextState = !app.showNewPageForm;
        closeAllDialogs();
        app.showNewPageForm = nextState;
        return;
      }

      // 3. Theme Toggle (Ctrl+T)
      if (isModKey(e, 't')) {
        e.preventDefault();
        const nextMode = mode.current === 'light' ? 'dark' : 'light';
        app.theme = nextMode;
        setMode(nextMode);
        saveUiSettings();
        return;
      }

      // 4. Help (F1)
      if (e.key === 'F1') {
        e.preventDefault();
        const nextState = !app.showHelp;
        closeAllDialogs();
        app.showHelp = nextState;
        return;
      }

      // Ignore other global shortcuts if any modal is open
      if (app.showSearch || app.showExitPrompt || app.showNewPageForm || app.showVaultSelector || app.showHelp || app.confirmDeleteSlug) return;

      // 5. Go Back (Alt+Left) - only when not in input
      if (e.altKey && (e.key === 'ArrowLeft' || e.key === 'Left') && !isInputFocused()) {
        e.preventDefault();
        goBack();
        return;
      }

      // 6. Go Forward (Alt+Right) - only when not in input
      if (e.altKey && (e.key === 'ArrowRight' || e.key === 'Right') && !isInputFocused()) {
        e.preventDefault();
        goForward();
        return;
      }

      // ── Preview Mode Shortcuts ──────────────────────────────────────────────────
      if (app.editorTab === 'preview' && !isInputFocused()) {
        // Switch pages
        if (e.key === 'j' || e.key === 'ArrowDown') {
          e.preventDefault();
          openNextPage();
          return;
        }
        if (e.key === 'k' || e.key === 'ArrowUp') {
          e.preventDefault();
          openPrevPage();
          return;
        }
        // Rename current page
        if (e.key === 'r') {
          e.preventDefault();
          triggerRename();
          return;
        }
        // Delete current page
        if (e.key === 'Delete' || e.key === 'Backspace') {
          e.preventDefault();
          triggerDelete();
          return;
        }
      }

      // 5. Global Search (Ctrl+P)
      if (isModKey(e, 'p')) {
        e.preventDefault();
        const nextState = !app.showSearch;
        closeAllDialogs();
        app.showSearch = nextState;
        return;
      }

      // 6. Add Vault (Ctrl+O)
      if (isModKey(e, 'o') && !e.shiftKey) {
        e.preventDefault();
        closeAllDialogs();
        void quickAddVault();
        return;
      }

      // 6. Vault Selector (Ctrl+Shift+O)
      if (isModKey(e, 'o') && e.shiftKey) {
        e.preventDefault();
        const nextState = !app.showVaultSelector;
        closeAllDialogs();
        app.showVaultSelector = nextState;
        return;
      }

      // 7. Quit App (Ctrl+Q)
      if (isModKey(e, 'q')) {
        e.preventDefault();
        closeAllDialogs();
        app.showExitPrompt = true;
        return;
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
    app.sidebarWidth = Math.max(200, Math.min(600, e.clientX));
  }

  function stopResizing() {
    isResizingSidebar = false;
    window.removeEventListener('mousemove', handleSidebarResize);
    window.removeEventListener('mouseup', stopResizing);
    saveUiSettings();
  }

  function toggleVaultSelector() {
    const nextState = !app.showVaultSelector;
    closeAllDialogs();
    app.showVaultSelector = nextState;
  }

  function toggleHelp() {
    const nextState = !app.showHelp;
    closeAllDialogs();
    app.showHelp = nextState;
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

<div class="app-shell" class:is-resizing={isResizingSidebar}>
  <!-- Sidebar -->
  <aside 
    class="sidebar" 
    style="width: {app.sidebarWidth}px;"
    bind:this={sidebarEl}
    tabindex="-1"
  >
    <!-- Vault header -->
    <div class="vault-header">
      <button
        class="vault-name-btn"
        onclick={toggleVaultSelector}
        title={`Switch vault (${modKeyLabel}+Shift+O)`}
        disabled={app.showNewPageForm}
      >
        <Vault size={18} class="vault-icon" />
        <span class="vault-label">{vault?.name ?? 'No vault'}</span>
        <span class="vault-chevron"><ChevronDown size={12} /></span>
      </button>

    </div>

    <!-- Page list -->
    <div class="page-list-wrap">
      {#if vault}
        <PageList />
      {:else}
        <div class="no-vault">
          <p>Add a vault to get started.</p>
          <button class="primary-btn" onclick={toggleVaultSelector}>Add Vault</button>
        </div>
      {/if}
    </div>

    <!-- Bottom actions area -->
    <div class="sidebar-footer">
        <button class="theme-icon-btn" onclick={toggleHelp} title="Keyboard shortcuts" aria-label="Keyboard shortcuts">
         <HelpCircle size={16} />
       </button>
       <button class="theme-toggle" onclick={() => {
         const nextMode = mode.current === 'light' ? 'dark' : 'light';
         app.theme = nextMode;
         setMode(nextMode);
         saveUiSettings();
       }} title="Toggle theme" aria-label="Toggle theme">
         {#if app.theme === 'dark'}
           <Moon size={18} />
         {:else}
           <Sun size={18} />
         {/if}
       </button>
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
</div>

<!-- ── Modals ──────────────────────────────────────────────────────────────── -->

{#if app.showSearch}
  <GlobalSearch onClose={() => app.showSearch = false} />
{/if}

{#if app.showNewPageForm}
  <NewPageDialog onClose={() => app.showNewPageForm = false} />
{/if}

{#if app.renameSlug}
  <RenamePageDialog onClose={() => app.renameSlug = null} />
{/if}

{#if app.showHelp}
  <KeyboardHelp onClose={() => app.showHelp = false} />
{/if}

{#if app.showVaultSelector}
  <div class="modal-backdrop" onclick={() => (app.showVaultSelector = false)} role="presentation">
    <div bind:this={vaultSelectorEl} class="vault-selector-modal" onkeydown={handleVaultSelectorKeydown} onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true" tabindex="-1" transition:fade={{ duration: 150 }}>
      <header class="modal-header">
        <h2>Switch Vault</h2>
        <button class="close-btn" onclick={() => (app.showVaultSelector = false)} title="Close (Esc)">✕</button>
      </header>

      <div class="vault-selector-body">
        {#each app.vaults as item (item.id)}
          <button class="vault-item" class:selected={item.id === app.vaults[selectedVaultIndex]?.id} onclick={() => selectVault(item.id)}>
            <Vault size={16} />
            <span class="vault-item-name">{item.name}</span>
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
    <button class="toast-close" onclick={() => (app.error = null)}>✕</button>
  </div>
{/if}

<style>
  .app-shell {
    display: flex;
    height: 100vh;
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

  .vault-header {
    position: relative;
    padding: 0 0.75rem;
    height: var(--header-height);
    display: flex;
    align-items: center;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .vault-name-btn {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    width: 100%;
    background: none;
    border: none;
    cursor: pointer;
    padding: 0.35rem 0.5rem;
    border-radius: var(--radius);
    font-size: 0.9rem;
    font-weight: 600;
    color: var(--fg);
    transition: background 0.15s;
  }

  .theme-icon-btn,
  .theme-toggle {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    padding: 0;
    background: none;
    border: none;
    cursor: pointer;
    border-radius: var(--radius);
    color: var(--muted-foreground);
    transition: background 0.15s, color 0.15s;
  }

  .theme-icon-btn:hover,
  .theme-toggle:hover {
    background: var(--accent-hover);
    color: var(--fg);
  }

  .vault-name-btn:hover {
    background: var(--accent-hover);
  }

  .vault-label {
    flex: 1;
    text-align: left;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .vault-chevron {
    font-size: 0.65rem;
    color: var(--muted-foreground);
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

  .vault-selector-body {
    padding: 0.5rem;
    max-height: 50vh;
    overflow-y: auto;
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
    background: var(--primary);
    color: #fff;
  }

  .vault-item-check {
    color: inherit;
    flex-shrink: 0;
  }

  .vault-item-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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

  .sidebar-footer {
    padding: 0.5rem;
    border-top: 1px solid var(--border);
    display: flex;
    justify-content: flex-start;
  }

  .theme-toggle {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    padding: 0;
    background: none;
    border: none;
    cursor: pointer;
    border-radius: var(--radius);
    color: var(--muted-foreground);
    transition: background 0.15s, color 0.15s;
  }

  .theme-toggle:hover {
    background: var(--accent-hover);
    color: var(--fg);
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
    font-size: 1rem;
    line-height: 1;
    padding: 0;
    flex-shrink: 0;
  }
</style>
