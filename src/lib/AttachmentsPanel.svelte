<script lang="ts">
  import { app, loadAttachments, type AttachmentInfo } from './stores.svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { Trash2, Paperclip, ExternalLink } from 'lucide-svelte';
  import { attachmentReference } from './attachment-drop';
  import ConfirmationDialog from './ConfirmationDialog.svelte';

  /**
   * The open page. Every action below works on an attachment's own UUID, never
   * on this one. `visible` is false while another right panel tab is selected:
   * the panel stays mounted but loads nothing then.
   */
  let { uuid, visible = true }: { uuid: string; visible?: boolean } = $props();
  let loading = $state(false);
  let confirmDelete = $state<AttachmentInfo | null>(null);
  let deleting = $state(false);

  async function refreshAttachments() {
    loading = true;
    await loadAttachments(uuid);
    loading = false;
  }

  async function deleteAttachment(attachment: AttachmentInfo) {
    if (deleting) return;
    deleting = true;
    try {
      await invoke('attachment_delete', { uuid: attachment.uuid });
      confirmDelete = null;
      await refreshAttachments();
    } catch (e) {
      confirmDelete = null;
      app.error = String(e);
    } finally {
      deleting = false;
    }
  }

  async function openAttachment(attachment: AttachmentInfo) {
    try {
      await invoke('attachment_open', { uuid: attachment.uuid });
    } catch (e) {
      app.error = String(e);
    }
  }

  function copyLink(attachment: AttachmentInfo) {
    void navigator.clipboard.writeText(attachmentReference(attachment.title, attachment.uuid));
  }

  $effect(() => {
    if (uuid && visible) {
      refreshAttachments();
    }
  });
</script>

<div class="attachments-panel">
  <!-- No header bar (spec `desktop-window-toolbar`): the title scrolls with
       the list. -->
  <div class="panel-content">
    <h3 class="panel-title">Attachments</h3>
    {#if loading}
      <p class="status">Loading...</p>
    {:else if app.attachments.length === 0}
      <p class="empty">No attachments for this page.</p>
    {:else}
      <ul class="attachment-list">
        {#each app.attachments as file (file.uuid)}
          <li class="attachment-item" data-uuid={file.uuid}>
            <button 
              class="file-info" 
              onclick={() => copyLink(file)}
              title="Click to copy markdown link"
            >
              <Paperclip size={14} class="icon" />
              <span class="filename">{file.title}</span>
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
                onclick={() => (confirmDelete = file)}
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

{#if confirmDelete}
  {@const target = confirmDelete}
  <ConfirmationDialog
    title="Delete attachment?"
    message={`"${target.title}" will be moved to the vault trash. Links to it in pages stay and stop working.`}
    confirmLabel={deleting ? 'Deleting…' : 'Delete'}
    confirmDisabled={deleting}
    onClose={() => (confirmDelete = null)}
    onConfirm={() => deleteAttachment(target)}
  />
{/if}

<style>
  .attachments-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--sidebar-bg);
    overflow: hidden;
  }

  .panel-title {
    margin: 0 0 0.5rem;
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
