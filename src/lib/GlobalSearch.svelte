<script lang="ts">
  import { onMount } from 'svelte';
  import { Search, FileText } from 'lucide-svelte';
  import { app, openPage, createPage, previewSlug } from './stores.svelte';
  import { isModKey, isMac } from './hotkeys';

  let { onClose }: { onClose: () => void } = $props();

  let query = $state('');
  let selectedIndex = $state(0);
  let inputEl = $state<HTMLInputElement | null>(null);

  // Reset selection when query changes
  $effect(() => {
    query;
    selectedIndex = 0;
  });

  async function handleCreate() {
    const title = query.trim();
    if (!title) return;
    
    try {
      // Check if slug already exists to satisfy "if no such slug" requirement
      const slug = await previewSlug(title);
      const existing = app.pages.find(p => p.slug === slug);
      if (existing) {
        openPage(existing.slug);
      } else {
        await createPage(title);
      }
      onClose();
    } catch (e) {
      console.error('Failed to create/open page:', e);
    }
  }

  onMount(() => {
    inputEl?.focus();

    async function handleWindowKeydown(e: KeyboardEvent) {
      // Prevent leak to editor
      e.stopPropagation();

      const list = results();

      // Ctrl/Cmd + Enter always tries to create/open exact match
      if (isModKey(e, 'Enter')) {
        e.preventDefault();
        await handleCreate();
        return;
      }

      if (e.key === 'ArrowDown') {
        e.preventDefault();
        selectedIndex = (selectedIndex + 1) % (list.length || 1);
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        selectedIndex = (selectedIndex - 1 + (list.length || 1)) % (list.length || 1);
      } else if (e.key === 'Enter') {
        e.preventDefault();
        if (list[selectedIndex]) {
          openPage(list[selectedIndex].slug);
          onClose();
        } else if (query.trim()) {
          // No results selected, create new page
          await handleCreate();
        }
      } else if (e.key === 'Escape') {
        e.preventDefault();
        onClose();
      }
    }

    window.addEventListener('keydown', handleWindowKeydown, true);
    return () => window.removeEventListener('keydown', handleWindowKeydown, true);
  });

  const results = $derived(() => {
    const q = query.toLowerCase().trim();
    if (!q) return app.pages.slice(0, 15);
    return app.pages
      .filter((p) => 
        p.title.toLowerCase().includes(q) || 
        p.slug.toLowerCase().includes(q)
      )
      .slice(0, 15);
  });
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div class="modal-backdrop" onclick={onClose} role="presentation">
  <div 
    class="search-modal" 
    onclick={(e) => e.stopPropagation()} 
    role="dialog" 
    aria-modal="true"
    tabindex="-1"
  >

    <div class="search-input-wrap">
      <Search size={18} class="search-icon" />
      <input
        bind:this={inputEl}
        bind:value={query}
        placeholder="Search pages..."
        class="search-input"
        spellcheck="false"
        autocomplete="off"
      />
    </div>

    <ul class="results">
      {#each results() as page, i}
        <li>
          <button
            class="result-item"
            class:selected={i === selectedIndex}
            onclick={() => { openPage(page.slug); onClose(); }}
          >
            <FileText size={16} class="result-icon" />
            <span class="result-title">{page.title}</span>
            <span class="result-slug">/{page.slug}</span>
          </button>
        </li>
      {/each}
      {#if results().length === 0}
        <li class="no-results">
          <p>No pages found matching "{query}"</p>
          {#if query.trim()}
            <button class="create-page-btn" onclick={handleCreate}>
              <FileText size={16} />
              Create page "{query}"
            </button>
          {/if}
        </li>
      {/if}
    </ul>

    <div class="search-footer">
      <span><kbd>↑↓</kbd> to navigate</span>
      <span><kbd>↵</kbd> to open</span>
      <span><kbd>{isMac ? '⌘' : 'Ctrl'} + ↵</kbd> to create</span>
      <span><kbd>esc</kbd> to close</span>
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

  .search-modal {
    width: 100%;
    max-width: 600px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg, 12px);
    box-shadow: 0 10px 40px rgba(0, 0, 0, 0.2);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .search-input-wrap {
    display: flex;
    align-items: center;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid var(--border);
    gap: 0.75rem;
  }

  .search-icon {
    color: var(--muted-foreground);
  }

  .search-input {
    flex: 1;
    background: transparent;
    border: none;
    font-size: 1.1rem;
    color: var(--fg);
    outline: none;
  }

  .results {
    list-style: none;
    padding: 0.5rem;
    margin: 0;
    max-height: 50vh;
    overflow-y: auto;
  }

  .result-item {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.6rem 0.75rem;
    border: none;
    background: transparent;
    border-radius: var(--radius);
    cursor: pointer;
    text-align: left;
    color: var(--fg);
    transition: all 0.1s;
  }

  .result-item.selected {
    background: var(--primary);
    color: #fff;
  }

  .result-item.selected .result-slug,
  .result-item.selected .result-icon {
    color: rgba(255, 255, 255, 0.8);
  }

  .result-icon {
    color: var(--muted-foreground);
    flex-shrink: 0;
  }

  .result-title {
    font-weight: 500;
    flex: 1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .result-slug {
    font-size: 0.75rem;
    color: var(--muted-foreground);
    font-family: monospace;
  }

  .no-results {
    padding: 2rem;
    text-align: center;
    color: var(--muted-foreground);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 1rem;
  }

  .no-results p {
    margin: 0;
  }

  .create-page-btn {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    background: var(--primary);
    color: white;
    border: none;
    padding: 0.5rem 1rem;
    border-radius: var(--radius);
    font-weight: 500;
    cursor: pointer;
    transition: opacity 0.2s;
  }

  .create-page-btn:hover {
    opacity: 0.9;
  }

  .search-footer {
    display: flex;
    gap: 1.5rem;
    padding: 0.5rem 1rem;
    background: var(--sidebar-bg);
    border-top: 1px solid var(--border);
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
</style>
