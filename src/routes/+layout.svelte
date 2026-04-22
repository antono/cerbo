<script lang="ts">
  import { onMount } from 'svelte';
  import { ModeWatcher, mode } from 'mode-watcher';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { Library } from 'lucide-svelte';
  import { app, loadVaults, openVault } from '$lib/stores.svelte';
  import VaultSwitcher from '$lib/VaultSwitcher.svelte';
  import PageList from '$lib/PageList.svelte';
  import ThemeToggle from '$lib/ThemeToggle.svelte';
  import '../app.css';

  let { children } = $props();
  let showVaultSwitcher = $state(false);

  onMount(async () => {
    await loadVaults();
    if (app.activeVaultId) {
      await openVault(app.activeVaultId);
    }
  });

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
</script>

<ModeWatcher />

<div class="app-shell">
  <!-- Sidebar -->
  <aside class="sidebar">
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
    {#if vault}
      <div class="page-list-wrap">
        <PageList />
      </div>
    {:else}
      <div class="no-vault">
        <p>Add a vault to get started.</p>
        <button onclick={() => (showVaultSwitcher = true)}>Add Vault</button>
      </div>
    {/if}

    <!-- Bottom actions area -->
    <div class="sidebar-footer">
       <ThemeToggle />
    </div>
  </aside>

  <!-- Main content -->
  <main class="main-area">
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
    width: var(--sidebar-width);
    min-width: var(--sidebar-width);
    background: var(--sidebar-bg);
    border-right: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .vault-header {
    position: relative;
    padding: 0.75rem;
    border-bottom: 1px solid var(--border);
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

  .no-vault button {
    padding: 0.4rem 0.9rem;
    background: var(--primary);
    color: #fff;
    border: none;
    border-radius: var(--radius);
    cursor: pointer;
    font-size: 0.875rem;
    transition: background 0.15s;
  }

  .no-vault button:hover {
    background: var(--primary-hover);
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
