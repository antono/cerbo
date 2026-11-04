<script lang="ts">
  /** "Rename folder" (spec `desktop-page-tree`, "Folder actions"). */
  import { onMount } from 'svelte';
  import { FolderPen } from 'lucide-svelte';
  import { app, renameFolder } from './stores.svelte';
  import { virtualPathError } from './virtual-path';

  let { onClose }: { onClose: () => void } = $props();

  const path = app.renameFolderPath ?? '';
  const segments = path.split('/');
  const parent = segments.slice(0, -1).join('/');
  let name = $state(segments[segments.length - 1]);
  let renaming = $state(false);
  let error = $state('');
  let inputEl = $state<HTMLInputElement | null>(null);

  const target = $derived(parent ? `${parent}/${name.trim()}` : name.trim());
  const validation = $derived(name.trim() === '' ? 'name must not be empty' : virtualPathError(target));
  const unchanged = $derived(target === path);

  async function handleRename() {
    if (renaming || validation !== null) return;
    if (unchanged) {
      onClose();
      return;
    }
    renaming = true;
    error = '';
    try {
      await renameFolder(path, target);
      onClose();
    } catch (e) {
      error = String(e);
    } finally {
      renaming = false;
    }
  }

  onMount(() => {
    inputEl?.focus();
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
  <div class="dialog-modal" onclick={(e) => e.stopPropagation()} role="dialog" aria-modal="true" tabindex="-1">
    <div class="header">
      <FolderPen size={18} class="icon" />
      <span class="title">Rename Folder</span>
    </div>

    <div class="input-wrap">
      <div class="current-meta">
        <div class="meta-item">
          <span class="label">Folder:</span>
          <span class="value">{path}</span>
        </div>
      </div>

      <div class="field">
        <label for="rename-folder" class="field-label">New name</label>
        <div class="name-row">
          {#if parent}<span class="parent">{parent}/</span>{/if}
          <input
            id="rename-folder"
            bind:this={inputEl}
            bind:value={name}
            placeholder="folder name"
            class="dialog-input"
            class:invalid={validation !== null}
            spellcheck="false"
            autocomplete="off"
            disabled={renaming}
          />
        </div>
        {#if validation}
          <div class="error-msg">{validation}</div>
        {:else if !unchanged}
          <div class="hint">Every page under <code>{path}</code> moves to <code>{target}</code>.</div>
        {/if}
      </div>

      {#if error}
        <div class="error-msg">{error}</div>
      {/if}
    </div>

    <div class="dialog-footer">
      <div class="shortcuts">
        <span><kbd>↵</kbd> to rename</span>
        <span><kbd>esc</kbd> to close</span>
      </div>
      <button type="button" class="primary-btn" onclick={handleRename} disabled={renaming || validation !== null}>
        {renaming ? 'Renaming…' : 'Rename'}
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
    word-break: break-all;
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
  .name-row {
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }
  .parent {
    color: var(--muted-foreground);
    font-size: 0.95rem;
    white-space: nowrap;
  }
  .dialog-input {
    flex: 1;
    min-width: 0;
    background: transparent;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 0.75rem 1rem;
    font-size: 1rem;
    color: var(--fg);
    outline: none;
    transition: border-color 0.2s;
  }
  .dialog-input:focus {
    border-color: var(--primary);
  }
  .dialog-input.invalid {
    border-color: #ef4444;
  }
  .hint {
    font-size: 0.8rem;
    color: var(--muted-foreground);
  }
  .hint code {
    font-family: ui-monospace, monospace;
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
