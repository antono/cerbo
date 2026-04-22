<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import { homeDir } from '@tauri-apps/api/path';
  import { Square, Plus, FolderOpen } from 'lucide-svelte';
  import { app, addVault, openVault, loadVaults } from './stores.svelte';

  let { onClose }: { onClose: () => void } = $props();

  let adding = $state(false);

  async function handleSelect(id: string) {
    if (id === app.activeVaultId) { onClose(); return; }
    await openVault(id);
    onClose();
  }

  async function quickAddVault() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        defaultPath: await homeDir(),
      });
      if (selected && typeof selected === 'string') {
        adding = true;
        const parts = selected.split(/[\\/]/);
        const last = parts.pop() || parts.pop(); // handle trailing slash
        const name = last || 'New Vault';
        
        await addVault(name, selected);
        onClose();
      }
    } catch (e) {
      console.error('Failed to add vault', e);
    } finally {
      adding = false;
    }
  }
</script>

<div class="vault-switcher">
  <div class="vault-list">
    {#each app.vaults as vault}
      <button
        class="vault-item"
        class:active={vault.id === app.activeVaultId}
        onclick={() => handleSelect(vault.id)}
      >
        <Square size={16} class="vault-icon" />
        <span class="vault-name">{vault.name}</span>
        {#if vault.id === app.activeVaultId}
          <span class="vault-check">✓</span>
        {/if}
      </button>
    {/each}
    {#if app.vaults.length === 0}
      <p class="empty-hint">No vaults yet.</p>
    {/if}
  </div>

  <div class="vault-footer">
    <button class="add-vault-bump" onclick={quickAddVault} disabled={adding}>
      <Plus size={16} /> 
      <span>{adding ? 'Adding…' : 'Add vault'}</span>
    </button>
  </div>
</div>

<style>
  .vault-switcher {
    display: flex;
    flex-direction: column;
    width: 100%;
    background: transparent;
    overflow: hidden;
  }
  .vault-list {
    padding: 0.25rem;
    display: flex;
    flex-direction: column;
    gap: 0.125rem;
  }
  .vault-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
    padding: 0.5rem 0.75rem;
    border: none;
    background: transparent;
    border-radius: 0.375rem;
    cursor: pointer;
    font-size: 0.875rem;
    text-align: left;
    color: inherit;
    min-width: 0;
  }
  .vault-item:hover { background: var(--accent-hover); }
  .vault-item.active { background: var(--accent); font-weight: 600; }
  .vault-name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .vault-check { margin-left: auto; flex-shrink: 0; color: var(--primary); }
  .vault-footer {
    border-top: 1px solid var(--border);
    padding: 0.75rem;
    background: var(--sidebar-bg);
  }
  .add-vault-bump {
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
  .add-vault-bump:hover {
    background: var(--primary-hover);
  }
  .add-vault-bump:active {
    transform: translateY(1px);
  }
  .add-form { display: flex; flex-direction: column; gap: 0.5rem; }
  .path-input-row {
    display: flex;
    gap: 0.25rem;
  }
  .btn-browse {
    padding: 0.25rem 0.5rem;
    background: var(--accent);
    border: 1px solid var(--border);
    border-radius: 0.375rem;
    cursor: pointer;
    font-size: 0.9rem;
    display: flex;
    align-items: center;
  }
  .btn-browse:hover { background: var(--accent-hover); }
  .add-form-actions { display: flex; gap: 0.375rem; }
  .input {
    padding: 0.5rem 0.75rem;
    font-size: 0.8125rem;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg);
    color: inherit;
    flex: 1;
    min-width: 0;
    box-sizing: border-box;
  }
  .btn-primary {
    padding: 0.5rem 1rem;
    background: var(--primary);
    color: #fff;
    border: none;
    border-radius: var(--radius);
    cursor: pointer;
    font-size: 0.8125rem;
    font-weight: 600;
    flex: 1;
  }
  .btn-primary:disabled { opacity: 0.6; }
  .btn-ghost {
    padding: 0.5rem 1rem;
    background: transparent;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    cursor: pointer;
    font-size: 0.8125rem;
    color: inherit;
    width: 100%;
  }
  .error { font-size: 0.75rem; color: #dc2626; margin: 0; }
  .empty-hint { padding: 0.5rem 0.75rem; font-size: 0.8125rem; color: var(--muted-foreground); margin: 0; }
</style>
