<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { Plus, FileText } from 'lucide-svelte';
  import { app, createPage, previewSlug, closeAllDialogs } from './stores.svelte';

  let { onClose }: { onClose: () => void } = $props();

  let title = $state('');
  let slugPreview = $state('');
  let creating = $state(false);
  let error = $state('');
  let inputEl = $state<HTMLInputElement | null>(null);

  async function handleCreate() {
    if (!title.trim() || creating) return;
    creating = true;
    error = '';
    try {
      await createPage(title.trim());
      onClose();
    } catch (e) {
      error = String(e);
    } finally {
      creating = false;
    }
  }

  $effect(() => {
    const t = title.trim();
    if (t) {
      previewSlug(t).then(s => slugPreview = s);
    } else {
      slugPreview = '';
    }
  });

  onMount(() => {
    inputEl?.focus();

    function handleWindowKeydown(e: KeyboardEvent) {
      if (e.key === 'Escape') {
        e.preventDefault();
        onClose();
      } else if (e.key === 'Enter') {
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
  <div 
    class="dialog-modal" 
    onclick={(e) => e.stopPropagation()} 
    role="dialog" 
    aria-modal="true"
    tabindex="-1"
  >
    <div class="header">
      <Plus size={18} class="icon" />
      <span class="title">New Page</span>
    </div>

    <div class="input-wrap">
      <input
        bind:this={inputEl}
        bind:value={title}
        placeholder="Enter page title..."
        class="dialog-input"
        spellcheck="false"
        autocomplete="off"
        disabled={creating}
      />
      {#if slugPreview}
        <div class="slug-preview">
          <span class="label">Slug:</span>
          <span class="slug">/{slugPreview}</span>
        </div>
      {/if}
      {#if error}
        <div class="error-msg">{error}</div>
      {/if}
    </div>

    <div class="dialog-footer">
      <div class="shortcuts">
        <span><kbd>↵</kbd> to create</span>
        <span><kbd>esc</kbd> to close</span>
      </div>
      {#if creating}
        <div class="loading">Creating...</div>
      {/if}
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
    overflow: hidden;
  }

  .header {
    display: flex;
    align-items: center;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid var(--border);
    gap: 0.75rem;
    background: var(--sidebar-bg);
  }

  .header .icon {
    color: var(--primary);
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
    gap: 0.75rem;
  }

  .dialog-input {
    width: 100%;
    background: transparent;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 0.75rem 1rem;
    font-size: 1.2rem;
    color: var(--fg);
    outline: none;
    transition: border-color 0.2s;
  }

  .dialog-input:focus {
    border-color: var(--primary);
  }

  .slug-preview {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.85rem;
    color: var(--muted-foreground);
  }

  .slug-preview .slug {
    font-family: monospace;
    color: var(--primary);
  }

  .error-msg {
    color: #ef4444;
    font-size: 0.85rem;
    margin-top: 0.25rem;
  }

  .dialog-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.75rem 1rem;
    background: var(--sidebar-bg);
    border-top: 1px solid var(--border);
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
    box-shadow: 0 1px 0 rgba(0,0,0,0.1);
  }

  .loading {
    font-size: 0.8rem;
    color: var(--primary);
    font-weight: 500;
  }
</style>
