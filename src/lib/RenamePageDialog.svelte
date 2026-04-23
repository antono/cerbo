<script lang="ts">
  import { onMount } from 'svelte';
  import { Pencil } from 'lucide-svelte';
  import { app, renamePage, previewSlug, closeAllDialogs } from './stores.svelte';

  let { onClose }: { onClose: () => void } = $props();

  let title = $state(app.renameTitle);
  let slugPreview = $state('');
  let renaming = $state(false);
  let error = $state('');
  let inputEl = $state<HTMLInputElement | null>(null);

  const currentSlug = app.renameSlug;
  const currentPage = $derived(app.pages.find(p => p.slug === currentSlug));

  async function handleRename() {
    if (!title.trim() || renaming || !currentSlug) return;
    renaming = true;
    error = '';
    try {
      await renamePage(currentSlug, title.trim());
      onClose();
    } catch (e) {
      error = String(e);
    } finally {
      renaming = false;
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
    // Select all text for easy replacement
    inputEl?.setSelectionRange(0, inputEl.value.length);

    function handleWindowKeydown(e: KeyboardEvent) {
      if (e.key === 'Escape') {
        e.preventDefault();
        onClose();
      } else if (e.key === 'Enter') {
        e.preventDefault();
        handleRename();
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
      <Pencil size={18} class="icon" />
      <span class="title">Rename Page</span>
    </div>

    <div class="input-wrap">
      <div class="current-meta">
        <div class="meta-item">
          <span class="label">Current Title:</span>
          <span class="value">{currentPage?.title ?? '...'}</span>
        </div>
        <div class="meta-item">
          <span class="label">Current Slug:</span>
          <span class="value slug">/{currentSlug}</span>
        </div>
      </div>

      <div class="field">
        <label for="rename-title" class="field-label">New Title</label>
        <input
          id="rename-title"
          bind:this={inputEl}
          bind:value={title}
          placeholder="Enter new title..."
          class="dialog-input"
          spellcheck="false"
          autocomplete="off"
          disabled={renaming}
        />
      </div>

      {#if slugPreview && slugPreview !== currentSlug}
        <div class="slug-preview">
          <span class="label">New Slug:</span>
          <span class="slug">/{slugPreview}</span>
        </div>
      {/if}
      
      {#if error}
        <div class="error-msg">{error}</div>
      {/if}
    </div>

    <div class="dialog-footer">
      <div class="shortcuts">
        <span><kbd>↵</kbd> to rename</span>
        <span><kbd>esc</kbd> to close</span>
      </div>
      {#if renaming}
        <div class="loading">Renaming...</div>
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
    gap: 1.25rem;
  }

  .current-meta {
    background: var(--accent-hover);
    padding: 0.75rem;
    border-radius: var(--radius);
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    font-size: 0.85rem;
  }

  .meta-item {
    display: flex;
    gap: 0.5rem;
  }

  .meta-item .label {
    color: var(--muted-foreground);
    width: 100px;
    flex-shrink: 0;
  }

  .meta-item .value {
    font-weight: 500;
  }

  .meta-item .slug {
    font-family: monospace;
    color: var(--primary);
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
