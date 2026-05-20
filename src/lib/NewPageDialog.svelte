<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { Plus } from 'lucide-svelte';
  import { app, createPage, closeAllDialogs } from './stores.svelte';
  import { stringToSlug } from './slug';

  let { onClose }: { onClose: () => void } = $props();

  let title = $state('');
  let slug = $state('');
  let virtualPath = $state('');
  let slugAutoUpdateEnabled = $state(true);
  let pathSuggestions = $state<string[]>([]);
  let showPathSuggestions = $state(false);
  let creating = $state(false);
  let error = $state('');
  let titleInputEl = $state<HTMLInputElement | null>(null);

  function updateSlugFromTitle() {
    if (slugAutoUpdateEnabled) {
      slug = stringToSlug(title);
    }
  }

  function onSlugInput() {
    slugAutoUpdateEnabled = false;
    showPathSuggestions = false;
  }

  function validateSlug(s: string): boolean {
    return /^[a-z0-9\-_]+$/.test(s);
  }

  function validateVirtualPath(p: string): boolean {
    return /^[a-z0-9\-_\/]+$/.test(p);
  }

  function onPathInput(value: string) {
    const existing = ['docs', 'docs/guides', 'docs/api', 'references', 'archive'];

    if (!value.trim()) {
      pathSuggestions = [];
      showPathSuggestions = false;
      return;
    }

    const lowerValue = value.toLowerCase();
    pathSuggestions = existing
      .filter(p => p.toLowerCase().startsWith(lowerValue))
      .sort();

    showPathSuggestions = pathSuggestions.length > 0;
  }

  function selectPathSuggestion(path: string) {
    virtualPath = path;
    showPathSuggestions = false;
  }

  function onInputKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter') {
      e.preventDefault();
      if (showPathSuggestions && pathSuggestions.length > 0) {
        selectPathSuggestion(pathSuggestions[0]);
      } else {
        handleCreate();
      }
    }
  }

  async function handleCreate() {
    if (!title.trim() || !slug.trim() || !virtualPath.trim() || creating) return;
    if (!validateSlug(slug) || !validateVirtualPath(virtualPath)) {
      error = 'Invalid slug or path format';
      return;
    }
    creating = true;
    error = '';
    try {
      await createPage(title.trim(), slug, virtualPath);
      onClose();
    } catch (e) {
      error = String(e);
    } finally {
      creating = false;
    }
  }

  onMount(() => {
    titleInputEl?.focus();

    function handleWindowKeydown(e: KeyboardEvent) {
      if (e.key === 'Escape') {
        e.preventDefault();
        onClose();
      } else if (e.key === 'Enter' && !showPathSuggestions) {
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
      <div class="field">
        <label>Page Title</label>
        <input
          bind:this={titleInputEl}
          bind:value={title}
          onchange={updateSlugFromTitle}
          onkeydown={onInputKeydown}
          placeholder="My Page..."
          class="dialog-input"
          spellcheck="false"
          autocomplete="off"
          disabled={creating}
        />
      </div>

      <div class="field">
        <label>Slug {#if !slugAutoUpdateEnabled}<span class="badge">manual</span>{/if}</label>
        <input
          bind:value={slug}
          oninput={(e) => {
            slug = e.currentTarget.value;
            onSlugInput();
          }}
          onchange={updateSlugFromTitle}
          onkeydown={onInputKeydown}
          placeholder="my-page"
          class="dialog-input"
          spellcheck="false"
          autocomplete="off"
          disabled={creating}
        />
        {#if slug && !validateSlug(slug)}
          <div class="error-msg">Only alphanumeric, hyphens, underscores</div>
        {/if}
      </div>

      <div class="field">
        <label>Mount Path</label>
        <div class="path-input-wrapper">
          <input
            bind:value={virtualPath}
            oninput={(e) => onPathInput(e.currentTarget.value)}
            onkeydown={onInputKeydown}
            placeholder="docs/guides"
            class="dialog-input"
            spellcheck="false"
            autocomplete="off"
            disabled={creating}
          />
          {#if showPathSuggestions && pathSuggestions.length > 0}
            <div class="suggestions">
              {#each pathSuggestions as suggestion}
                <button
                  class="suggestion-item"
                  onclick={() => selectPathSuggestion(suggestion)}
                  disabled={creating}
                >
                  {suggestion}
                </button>
              {/each}
            </div>
          {/if}
        </div>
        {#if virtualPath && !validateVirtualPath(virtualPath)}
          <div class="error-msg">Only alphanumeric, hyphens, underscores, slashes</div>
        {/if}
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
    gap: 1rem;
    max-height: 60vh;
    overflow-y: auto;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .field label {
    font-size: 0.85rem;
    font-weight: 500;
    color: var(--muted-foreground);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .badge {
    display: inline-block;
    font-size: 0.7rem;
    background: var(--primary);
    color: white;
    padding: 0.15rem 0.4rem;
    border-radius: 3px;
    margin-left: 0.5rem;
    font-weight: 600;
    text-transform: none;
    letter-spacing: normal;
  }

  .dialog-input {
    width: 100%;
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

  .path-input-wrapper {
    position: relative;
  }

  .suggestions {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    background: var(--bg);
    border: 1px solid var(--border);
    border-top: none;
    border-radius: 0 0 var(--radius) var(--radius);
    max-height: 200px;
    overflow-y: auto;
    z-index: 10;
    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
  }

  .suggestion-item {
    display: block;
    width: 100%;
    padding: 0.75rem 1rem;
    text-align: left;
    background: transparent;
    border: none;
    border-bottom: 1px solid var(--border);
    color: var(--fg);
    cursor: pointer;
    font-size: 0.95rem;
    transition: background-color 0.15s;
  }

  .suggestion-item:last-child {
    border-bottom: none;
  }

  .suggestion-item:hover:not(:disabled) {
    background-color: var(--sidebar-bg);
  }

  .suggestion-item:disabled {
    opacity: 0.5;
    cursor: not-allowed;
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
