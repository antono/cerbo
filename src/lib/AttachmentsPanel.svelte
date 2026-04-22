<script lang="ts">
  import { app } from './stores.svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { FileText, Trash2, Paperclip } from 'lucide-svelte';

  let { slug }: { slug: string } = $props();
  let attachments = $state<string[]>([]);
  let loading = $state(false);

  async function loadAttachments() {
    if (!app.activeVaultId || !slug) return;
    loading = true;
    try {
      attachments = await invoke<string[]>('attachment_list', {
        vaultId: app.activeVaultId,
        slug
      });
    } catch (e) {
      console.error('Failed to load attachments:', e);
    } finally {
      loading = false;
    }
  }

  async function deleteAttachment(filename: string) {
    if (!app.activeVaultId || !slug) return;
    if (!confirm(`Are you sure you want to delete "${filename}"?`)) return;

    try {
      await invoke('attachment_delete', {
        vaultId: app.activeVaultId,
        slug,
        filename
      });
      await loadAttachments();
    } catch (e) {
      app.error = String(e);
    }
  }

  function insertLink(filename: string) {
    // This would ideally interact with the editor to insert markdown
    // For now, we just copy it to clipboard or alert
    const md = `[${filename}](assets/${filename})`;
    navigator.clipboard.writeText(md);
    // TODO: Event to insert into editor
  }

  $effect(() => {
    if (slug) {
      loadAttachments();
    }
  });
</script>

<div class="attachments-panel">
  <div class="panel-header">
    <h3 class="panel-title">Attachments</h3>
  </div>

  <div class="panel-content">
    {#if loading}
      <p class="status">Loading...</p>
    {:else if attachments.length === 0}
      <p class="empty">No attachments for this page.</p>
    {:else}
      <ul class="attachment-list">
        {#each attachments as file}
          <li class="attachment-item">
            <button 
              class="file-info" 
              onclick={() => insertLink(file)}
              title="Click to copy markdown link"
            >
              <Paperclip size={14} class="icon" />
              <span class="filename">{file}</span>
            </button>
            <button 
              class="delete-btn" 
              onclick={() => deleteAttachment(file)}
              title="Delete attachment"
            >
              <Trash2 size={14} />
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</div>

<style>
  .attachments-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--sidebar-bg);
    overflow: hidden;
  }

  .panel-header {
    height: var(--header-height);
    display: flex;
    align-items: center;
    padding: 0 1rem;
    flex-shrink: 0;
  }

  .panel-title {
    margin: 0;
    font-size: 0.7rem;
    font-weight: 700;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--muted-foreground);
    flex: 1;
  }

  .panel-content {
    flex: 1;
    overflow-y: auto;
    padding: 1rem;
  }

  .status, .empty {
    margin: 0;
    color: var(--muted-foreground);
    font-size: 0.8125rem;
    font-style: italic;
  }

  .attachment-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .attachment-item {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.25rem;
    border-radius: var(--radius);
    transition: background 0.15s;
  }

  .attachment-item:hover {
    background: var(--accent-hover);
  }

  .file-info {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    background: none;
    border: none;
    padding: 0.25rem 0.4rem;
    cursor: pointer;
    text-align: left;
    min-width: 0;
    color: var(--fg);
    font-size: 0.8125rem;
  }

  :global(.attachment-item .lucide) {
    flex-shrink: 0;
    color: var(--muted-foreground);
  }

  .filename {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .delete-btn {
    background: none;
    border: none;
    color: var(--muted-foreground);
    cursor: pointer;
    padding: 0.4rem;
    border-radius: var(--radius);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s;
    opacity: 0;
  }

  .attachment-item:hover .delete-btn {
    opacity: 1;
  }

  .delete-btn:hover {
    background: #fee2e2;
    color: #dc2626;
  }

  :global(.dark) .delete-btn:hover {
    background: #450a0a;
    color: #f87171;
  }
</style>
