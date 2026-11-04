<script lang="ts">
  /** "Move to…" (spec `desktop-page-tree`, "Move page dialog"). */
  import { onMount } from 'svelte';
  import { FolderInput } from 'lucide-svelte';
  import { app, movePage } from './stores.svelte';
  import VirtualPathInput from './VirtualPathInput.svelte';
  import { normalizeVirtualPath, virtualPathError } from './virtual-path';

  let { onClose }: { onClose: () => void } = $props();

  const uuid = app.moveUuid;
  const page = $derived(app.pages.find((p) => p.uuid === uuid));
  let value = $state(app.pages.find((p) => p.uuid === uuid)?.virtualPath ?? '');
  let suggestionsOpen = $state(false);
  let inputEl = $state<HTMLInputElement | null>(null);
  let moving = $state(false);
  let error = $state('');

  const invalid = $derived(virtualPathError(value.trim()) !== null);
  const unchanged = $derived(normalizeVirtualPath(value) === (page?.virtualPath ?? null));

  async function handleMove() {
    if (!uuid || moving || invalid) return;
    if (unchanged) {
      onClose();
      return;
    }
    moving = true;
    error = '';
    try {
      await movePage(uuid, normalizeVirtualPath(value));
      onClose();
    } catch (e) {
      error = String(e);
    } finally {
      moving = false;
    }
  }

  onMount(() => {
    inputEl?.focus();
    inputEl?.setSelectionRange(0, inputEl.value.length);

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
        handleMove();
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
      <FolderInput size={18} class="icon" />
      <span class="title">Move Page</span>
    </div>

    <div class="input-wrap">
      <div class="current-meta">
        <div class="meta-item">
          <span class="label">Page:</span>
          <span class="value">{page?.title ?? '…'}</span>
        </div>
        <div class="meta-item">
          <span class="label">Now in:</span>
          <span class="value">{page?.virtualPath ?? '(root)'}</span>
        </div>
      </div>

      <div class="field">
        <label for="move-path" class="field-label">Move to <span class="optional">(empty = root)</span></label>
        <VirtualPathInput id="move-path" bind:value bind:suggestionsOpen bind:inputEl disabled={moving} />
      </div>

      {#if error}
        <div class="error-msg">{error}</div>
      {/if}
    </div>

    <div class="dialog-footer">
      <div class="shortcuts">
        <span><kbd>↵</kbd> to move</span>
        <span><kbd>esc</kbd> to close</span>
      </div>
      <button type="button" class="primary-btn" onclick={handleMove} disabled={moving || invalid}>
        {moving ? 'Moving…' : 'Move'}
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
  .optional {
    font-weight: 400;
    text-transform: none;
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
