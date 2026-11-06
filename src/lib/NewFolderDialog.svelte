<script lang="ts">
  /** "New folder" (spec `desktop-tree-editing`, "New folder dialog"). */
  import { onMount } from 'svelte';
  import { FolderPlus } from 'lucide-svelte';
  import { app, createFolder } from './stores.svelte';
  import VirtualPathInput from './VirtualPathInput.svelte';
  import { normalizeVirtualPath, virtualPathError } from './virtual-path';

  let { onClose }: { onClose: () => void } = $props();

  let value = $state(app.newFolderPath);
  let suggestionsOpen = $state(false);
  let inputEl = $state<HTMLInputElement | null>(null);
  let creating = $state(false);
  let error = $state('');

  const path = $derived(normalizeVirtualPath(value));
  const invalid = $derived(path === null || virtualPathError(value.trim()) !== null);

  async function handleCreate() {
    if (creating || invalid || path === null) return;
    creating = true;
    error = '';
    try {
      await createFolder(path);
      onClose();
    } catch (e) {
      error = String(e);
    } finally {
      creating = false;
    }
  }

  onMount(() => {
    inputEl?.focus();
    // Keep a prefilled "parent/" and put the caret after it.
    inputEl?.setSelectionRange(inputEl.value.length, inputEl.value.length);

    function handleWindowKeydown(e: KeyboardEvent) {
      if (e.key === 'Escape') {
        e.preventDefault();
        if (suggestionsOpen) {
          // Consumed here; the layout's Escape must not close the dialog.
          e.stopPropagation();
          suggestionsOpen = false;
        } else {
          onClose();
        }
      } else if (e.key === 'Enter' && !suggestionsOpen) {
        e.preventDefault();
        handleCreate();
      }
    }

    window.addEventListener('keydown', handleWindowKeydown, true);
    return () => window.removeEventListener('keydown', handleWindowKeydown, true);
  });
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div class="modal-backdrop" onclick={onClose} role="presentation">
  <div class="dialog-modal" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true" tabindex="-1">
    <div class="header">
      <FolderPlus size={18} class="icon" />
      <span class="title">New Folder</span>
    </div>

    <div class="input-wrap">
      <div class="field">
        <label for="new-folder-path" class="field-label">Folder path</label>
        <VirtualPathInput id="new-folder-path" bind:value bind:suggestionsOpen bind:inputEl disabled={creating} />
        <div class="hint">The folder stays in the sidebar while empty, until you delete it.</div>
      </div>

      {#if error}
        <div class="error-msg">{error}</div>
      {/if}
    </div>

    <div class="dialog-footer">
      <div class="shortcuts">
        <span><kbd>↵</kbd> to create</span>
        <span><kbd>esc</kbd> to close</span>
      </div>
      <button type="button" class="primary-btn" onclick={handleCreate} disabled={creating || invalid}>
        {creating ? 'Creating…' : 'Create'}
      </button>
    </div>
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    backdrop-filter: blur(2px);
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 20vh;
    z-index: 1000;
  }
  .dialog-modal {
    width: 100%;
    max-width: 500px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg, 12px);
    box-shadow: 0 10px 40px rgba(0, 0, 0, 0.2);
    display: flex;
    flex-direction: column;
    overflow: visible;
  }
  .header {
    display: flex;
    align-items: center;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid var(--border);
    gap: 0.75rem;
    background: var(--sidebar-bg);
    border-radius: var(--radius-lg, 12px) var(--radius-lg, 12px) 0 0;
  }
  .header .title {
    font-size: 0.9rem;
    font-weight: 600;
    color: var(--muted-foreground);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .input-wrap {
    padding: 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .field-label {
    font-size: 0.8rem;
    font-weight: 600;
    color: var(--muted-foreground);
    text-transform: uppercase;
  }
  .hint {
    font-size: 0.8rem;
    color: var(--muted-foreground);
  }
  .error-msg {
    color: #ef4444;
    font-size: 0.85rem;
  }
  .dialog-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.75rem 1rem;
    background: var(--sidebar-bg);
    border-top: 1px solid var(--border);
    border-radius: 0 0 var(--radius-lg, 12px) var(--radius-lg, 12px);
  }
  .shortcuts {
    display: flex;
    gap: 1.5rem;
    font-size: 0.7rem;
    color: var(--muted-foreground);
  }
  kbd {
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 0 4px;
    font-family: inherit;
    font-size: 0.8em;
    box-shadow: 0 1px 0 rgba(0, 0, 0, 0.1);
  }
  .primary-btn {
    background: var(--primary);
    color: var(--primary-foreground);
    border: none;
    border-radius: var(--radius);
    padding: 0.4rem 0.9rem;
    font-size: 0.85rem;
    font-weight: 500;
    cursor: pointer;
  }
  .primary-btn:hover:not(:disabled) {
    background: var(--primary-hover);
  }
  .primary-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
