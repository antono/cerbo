<script lang="ts">
  import { onMount } from 'svelte';
  import { app, loadVaults, openVault, activeVault } from '$lib/stores.svelte';
  import VaultSwitcher from '$lib/VaultSwitcher.svelte';
  import PageList from '$lib/PageList.svelte';
  import PageEditor from '$lib/PageEditor.svelte';
  import BacklinksPanel from '$lib/BacklinksPanel.svelte';

  // ── State ─────────────────────────────────────────────────────────────────────

  let showVaultSwitcher = $state(false);

  // ── Init: load vaults and open active vault on startup ───────────────────────

  onMount(async () => {
    await loadVaults();
    if (app.activeVaultId) {
      await openVault(app.activeVaultId);
    }
  });

  // ── Derived ───────────────────────────────────────────────────────────────────

  let vault = $derived(activeVault());
  let currentPage = $derived(
    app.pages.find((p) => p.slug === app.currentSlug) ?? null,
  );
</script>

<!-- ── App shell ───────────────────────────────────────────────────────────── -->
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
        <span class="vault-icon">🗂</span>
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
  </aside>

  <!-- Main content -->
  <main class="main-area">
    {#if app.loading}
      <div class="loading-overlay">
        <div class="loading-spinner"></div>
        <p>{app.loadingMessage || 'Loading…'}</p>
      </div>
    {:else if app.currentSlug}
      <div class="editor-area">
        <!-- Page title bar -->
        <div class="page-title-bar">
          <h1 class="page-title">{currentPage?.title ?? app.currentSlug}</h1>
        </div>

        <!-- Editor -->
        <div class="editor-wrap">
          <PageEditor slug={app.currentSlug} />
        </div>

        <!-- Backlinks -->
        <BacklinksPanel slug={app.currentSlug} />
      </div>
    {:else if vault}
      <div class="empty-state">
        <p>Select a page from the sidebar, or create a new one.</p>
      </div>
    {:else}
      <div class="empty-state">
        <p>Welcome to Cerbo. Add a vault to begin.</p>
        <button class="btn-primary" onclick={() => (showVaultSwitcher = true)}>
          Add Vault
        </button>
      </div>
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
  /* ── Layout ──────────────────────────────────────────────────────────────── */

  :global(*, *::before, *::after) {
    box-sizing: border-box;
  }

  :global(html, body) {
    margin: 0;
    padding: 0;
    height: 100%;
    font-family: Inter, system-ui, -apple-system, sans-serif;
    font-size: 15px;
    background: var(--bg, #ffffff);
    color: var(--fg, #1a1a1a);
  }

  :global(:root) {
    --bg: #ffffff;
    --fg: #1a1a1a;
    --border: #e2e8f0;
    --sidebar-bg: #f8fafc;
    --accent: #f1f5f9;
    --accent-hover: #e2e8f0;
    --primary: #3b82f6;
    --primary-hover: #2563eb;
    --muted-foreground: #64748b;
    --radius: 0.375rem;
    --sidebar-width: 260px;
  }

  @media (prefers-color-scheme: dark) {
    :global(:root) {
      --bg: #0f1117;
      --fg: #e8eaed;
      --border: #2d3748;
      --sidebar-bg: #161b22;
      --accent: #1e2535;
      --accent-hover: #252d3d;
      --primary: #60a5fa;
      --primary-hover: #93c5fd;
      --muted-foreground: #94a3b8;
    }
  }

  .app-shell {
    display: flex;
    height: 100vh;
    overflow: hidden;
  }

  /* ── Sidebar ─────────────────────────────────────────────────────────────── */

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

  /* ── Main area ───────────────────────────────────────────────────────────── */

  .main-area {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    background: var(--bg);
  }

  .editor-area {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }

  .page-title-bar {
    padding: 0.75rem 1.5rem 0.5rem;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .page-title {
    margin: 0;
    font-size: 1.2rem;
    font-weight: 700;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .editor-wrap {
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: var(--muted-foreground);
    gap: 1rem;
    font-size: 0.95rem;
  }

  .btn-primary {
    padding: 0.5rem 1.25rem;
    background: var(--primary);
    color: #fff;
    border: none;
    border-radius: var(--radius);
    cursor: pointer;
    font-size: 0.9rem;
    transition: background 0.15s;
  }

  .btn-primary:hover {
    background: var(--primary-hover);
  }

  /* ── Loading overlay ─────────────────────────────────────────────────────── */

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

  /* ── Error toast ─────────────────────────────────────────────────────────── */

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
