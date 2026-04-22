<script lang="ts">
  import { onMount } from 'svelte';
  import { ModeWatcher, mode } from 'mode-watcher';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { Library } from 'lucide-svelte';
  import { app, loadVaults, openVault, quitApp } from '$lib/stores.svelte';
  import VaultSwitcher from '$lib/VaultSwitcher.svelte';
  import PageList from '$lib/PageList.svelte';
  import ThemeToggle from '$lib/ThemeToggle.svelte';
  import GlobalSearch from '$lib/GlobalSearch.svelte';
  import ExitConfirmation from '$lib/ExitConfirmation.svelte';
  import { isModKey, isModArrow, isInputFocused } from '$lib/hotkeys';
  import '../app.css';

  let { children } = $props();
  let showVaultSwitcher = $state(false);

  // Focusable panel refs
  let sidebarEl = $state<HTMLElement | null>(null);
  let mainEl = $state<HTMLElement | null>(null);

  onMount(async () => {
    await loadVaults();
    if (app.activeVaultId) {
      await openVault(app.activeVaultId);
    }
  });

  // Global keydown listener
  $effect(() => {
    function handleKeydown(e: KeyboardEvent) {
      // 1. Global Search (Ctrl+P)
      if (isModKey(e, 'p')) {
        e.preventDefault();
        app.showSearch = !app.showSearch;
        return;
      }

      // 2. Quit App (Ctrl+Q)
      if (isModKey(e, 'q')) {
        e.preventDefault();
        app.showExitPrompt = true;
        return;
      }

      // 3. Panel Navigation (Ctrl+Arrows)
      const arrow = isModArrow(e);
      if (arrow) {
        e.preventDefault();
        navigatePanels(arrow === 'ArrowLeft' ? -1 : 1);
        return;
      }
    }

    window.addEventListener('keydown', handleKeydown);
    return () => window.removeEventListener('keydown', handleKeydown);
  });

  function navigatePanels(direction: number) {
    const panels: ('sidebar' | 'editor' | 'panels')[] = ['sidebar', 'editor', 'panels'];
    let currentIdx = panels.indexOf(app.activePanel);
    let nextIdx = (currentIdx + direction + panels.length) % panels.length;
    
    // Skip panels if side panels are hidden
    if (panels[nextIdx] === 'panels' && !app.backlinksVisible) {
      nextIdx = (nextIdx + direction + panels.length) % panels.length;
    }

    app.activePanel = panels[nextIdx];

    // Focus corresponding element
    if (app.activePanel === 'sidebar') {
      sidebarEl?.focus();
    } else if (app.activePanel === 'editor') {
      mainEl?.focus();
    } else if (app.activePanel === 'panels') {
      // This is in +page.svelte, we'll need a way to focus it.
      // For now, let's use a query selector or a store ref if needed.
      document.querySelector<HTMLElement>('.right-panels')?.focus();
    }
  }

  $effect(() => {
    // Synchronize Tauri window theme with app theme
    if (mode.current) {
      const win = getCurrentWindow();
      win.setTheme(mode.current === 'dark' ? 'dark' : 'light').catch(() => {
        // Ignore errors if not running in Tauri or permission denied
      });
    }
  });

  let vault = $derived(app.vaults.find((v) => v.id === app.activeVaultId));

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
    onfocus={() => app.activePanel = 'sidebar'}
  >
    <!-- Vault header -->
    <div class="vault-header">
      <button
        class="vault-name-btn"
        onclick={() => (showVaultSwitcher = !showVaultSwitcher)}
        title="Switch vault"
      >
        <Library size={18} class="vault-icon" />
        <span class="vault-label">{vault?.name ?? 'No vault'}</span>
        <span class="vault-chevron">{showVaultSwitcher ? '▲' : '▼'}</span>
      </button>

      {#if showVaultSwitcher}
        <div class="vault-switcher-popup">
          <VaultSwitcher onClose={() => (showVaultSwitcher = false)} />
        </div>
      {/if}
    </div>

    <!-- Page list -->
    <div class="page-list-wrap">
      {#if vault}
        <PageList />
      {:else}
        <div class="no-vault">
          <p>Add a vault to get started.</p>
          <button class="primary-btn" onclick={() => (showVaultSwitcher = true)}>Add Vault</button>
        </div>
      {/if}
    </div>

    <!-- Bottom actions area -->
    <div class="sidebar-footer">
       <ThemeToggle />
    </div>
  </aside>

  <!-- Resize Handle -->
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
    onfocus={() => app.activePanel = 'editor'}
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

  .vault-switcher-popup {
    position: absolute;
    top: calc(100% + 4px);
    left: 0.75rem;
    right: 0.75rem;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: 0 4px 16px rgba(0,0,0,0.12);
    z-index: 100;
    max-height: 60vh;
    overflow-y: auto;
    overflow-x: hidden;
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
