<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import { homeDir } from '@tauri-apps/api/path';
  import { app, addVault, openVault, loadVaults } from './stores.svelte';

  let { onClose }: { onClose: () => void } = $props();

  let showAddForm = $state(false);
  let newName = $state('');
  let newPath = $state('');
  let adding = $state(false);
  let addError = $state('');

  async function handleSelect(id: string) {
    if (id === app.activeVaultId) { onClose(); return; }
    await openVault(id);
    onClose();
  }

  async function handleAdd() {
    if (!newName.trim() || !newPath.trim()) return;
    adding = true;
    addError = '';
    try {
      await addVault(newName.trim(), newPath.trim());
      newName = '';
      newPath = '';
      showAddForm = false;
      onClose();
    } catch (e) {
      addError = String(e);
    } finally {
      adding = false;
    }
  }

  async function selectFolder() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        defaultPath: await homeDir(),
      });
      if (selected && typeof selected === 'string') {
        newPath = selected;
        // If name is empty, try to use the folder name
        if (!newName.trim()) {
          const parts = selected.split(/[\\/]/);
          const last = parts.pop() || parts.pop(); // handle trailing slash
          if (last) newName = last;
        }
      }
    } catch (e) {
      console.error('Failed to open dialog', e);
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
        <span class="vault-icon">⊞</span>
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
    {#if showAddForm}
      <div class="add-form">
        <input
          class="input"
          placeholder="Vault name"
          bind:value={newName}
          onkeydown={(e) => e.key === 'Enter' && handleAdd()}
        />
        <div class="path-input-row">
          <input
            class="input"
            placeholder="/path/to/folder"
            bind:value={newPath}
            onkeydown={(e) => e.key === 'Enter' && handleAdd()}
          />
          <button class="btn-browse" onclick={selectFolder} title="Browse">
            📂
          </button>
        </div>
        {#if addError}
          <p class="error">{addError}</p>
        {/if}
        <div class="add-form-actions">
          <button class="btn-primary" onclick={handleAdd} disabled={adding}>
            {adding ? 'Adding…' : 'Add'}
          </button>
          <button class="btn-ghost" onclick={() => { showAddForm = false; addError = ''; }}>
            Cancel
          </button>
        </div>
      </div>
    {:else}
      <button class="btn-ghost add-btn" onclick={() => { showAddForm = true; }}>
        + Add vault
      </button>
    {/if}
  </div>
</div>

<style>
  .vault-switcher {
    display: flex;
    flex-direction: column;
    min-width: 240px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 0.5rem;
    box-shadow: 0 4px 16px rgba(0,0,0,0.12);
    overflow: hidden;
  }
  .vault-list {
    padding: 0.25rem;
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
  }
  .vault-item:hover { background: var(--accent-hover); }
  .vault-item.active { background: var(--accent); font-weight: 600; }
  .vault-check { margin-left: auto; color: var(--primary); }
  .vault-footer {
    border-top: 1px solid var(--border);
    padding: 0.5rem;
  }
  .add-form { display: flex; flex-direction: column; gap: 0.375rem; }
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
  }
  .btn-browse:hover { background: var(--accent-hover); }
  .add-form-actions { display: flex; gap: 0.375rem; }
  .input {
    padding: 0.375rem 0.5rem;
    font-size: 0.8125rem;
    border: 1px solid var(--border);
    border-radius: 0.375rem;
    background: var(--bg);
    color: inherit;
    width: 100%;
    box-sizing: border-box;
  }
  .btn-primary {
    padding: 0.375rem 0.75rem;
    background: var(--primary);
    color: #fff;
    border: none;
    border-radius: 0.375rem;
    cursor: pointer;
    font-size: 0.8125rem;
    flex: 1;
  }
  .btn-primary:disabled { opacity: 0.6; }
  .btn-ghost {
    padding: 0.375rem 0.75rem;
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 0.375rem;
    cursor: pointer;
    font-size: 0.8125rem;
    color: inherit;
    width: 100%;
  }
  .add-btn { margin-top: 0; }
  .error { font-size: 0.75rem; color: #dc2626; margin: 0; }
  .empty-hint { padding: 0.5rem 0.75rem; font-size: 0.8125rem; color: var(--muted-foreground); margin: 0; }
</style>
