<script lang="ts">
  import { app, loadAttachments } from './stores.svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { FileText, Trash2, Paperclip, ExternalLink } from 'lucide-svelte';

  let { slug }: { slug: string } = $props();
  let loading = $state(false);

  async function refreshAttachments() {
    loading = true;
    await loadAttachments(slug);
    loading = false;
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
      await refreshAttachments();
    } catch (e) {
      app.error = String(e);
    }
  }

  async function openAttachment(filename: string) {
    if (!app.activeVaultId || !slug) return;
    try {
      await invoke('attachment_open', {
        vaultId: app.activeVaultId,
        slug,
        filename
      });
    } catch (e) {
      app.error = String(e);
    }
  }

  function insertLink(filename: string) {
    const isImage = /\.(png|jpe?g|gif|svg|webp)$/i.test(filename);
    const encoded = encodeURIComponent(filename).replace(/%20/g, '%20');
    // Using <...> wrapper is also an option for spaces, but %20 is more compatible
    const md = `${isImage ? '!' : ''}[${filename}](assets/${encoded})`;
    navigator.clipboard.writeText(md);
  }

  $effect(() => {
    if (slug) {
      refreshAttachments();
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
    {:else if app.attachments.length === 0}
      <p class="empty">No attachments for this page.</p>
    {:else}
      <ul class="attachment-list">
        {#each app.attachments as file}
          <li class="attachment-item">
            <button 
              class="file-info" 
              onclick={() => insertLink(file)}
              title="Click to copy markdown link"
            >
              <Paperclip size={14} class="icon" />
              <span class="filename">{file}</span>
            </button>
            <div class="actions">
              <button 
                class="action-btn open-btn" 
                onclick={() => openAttachment(file)}
                title="Open with system default app"
              >
                <ExternalLink size={14} />
              </button>
              <button 
                class="action-btn delete-btn" 
                onclick={() => deleteAttachment(file)}
                title="Delete attachment"
              >
                <Trash2 size={14} />
              </button>
            </div>
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

  .actions {
    display: flex;
    gap: 0.125rem;
    opacity: 0;
    transition: opacity 0.15s;
  }

  .attachment-item:hover .actions {
    opacity: 1;
  }

  .action-btn {
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
  }

  .open-btn:hover {
    background: var(--accent-hover);
    color: var(--primary);
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
