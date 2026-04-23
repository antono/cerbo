<script lang="ts">
  import { FileText, Plus, Pencil, Trash2, X, Search } from 'lucide-svelte';
  import { tick } from 'svelte';
  import { app, openPage, createPage, deletePage, renamePage, previewSlug, closeAllDialogs, triggerRename, triggerDelete, openNextPage, openPrevPage } from './stores.svelte';
  import { isInputFocused } from './hotkeys';

  // ── State for dialogs ─────────────────────────────────────────────────────────

  let deleting = $state(false);

  let renameSlugPreview = $state('');
  let renaming = $state(false);
  let renameError = $state('');

  // ── Focus handling ────────────────────────────────────────────────────────────

  let itemsList = $state<HTMLUListElement | null>(null);

  function handleListKeydown(e: KeyboardEvent) {
    const isJorK = e.key === 'j' || e.key === 'k';
    const isArrow = e.key === 'ArrowDown' || e.key === 'ArrowUp';
    const isTab = e.key === 'Tab';
    const isR = e.key === 'r';
    const isDel = e.key === 'Delete' || e.key === 'Backspace';

    if (!isJorK && !isArrow && !isTab && !isR && !isDel) return;

    // Navigation keys should only work if no input is focused
    if (isInputFocused()) return;

    const buttons = Array.from(itemsList?.querySelectorAll('.page-btn') || []) as HTMLButtonElement[];
    if (buttons.length === 0) return;

    const focusedElement = document.activeElement as HTMLButtonElement;
    const currentIndex = buttons.indexOf(focusedElement);

    if (isR) {
      e.preventDefault();
      const slug = currentIndex !== -1 ? app.pages[currentIndex].slug : app.currentSlug;
      if (slug) triggerRename(slug);
      return;
    }

    if (isDel) {
      e.preventDefault();
      const slug = currentIndex !== -1 ? app.pages[currentIndex].slug : app.currentSlug;
      if (slug) triggerDelete(slug);
      return;
    }

    if (e.key === 'ArrowDown' || e.key === 'j' || (e.key === 'Tab' && !e.shiftKey)) {
      e.preventDefault();
      openNextPage();
    } else if (e.key === 'ArrowUp' || e.key === 'k' || (e.key === 'Tab' && e.shiftKey)) {
      e.preventDefault();
      openPrevPage();
    }
  }

  async function onRenameTitleInput() {
    if (app.renameTitle.trim()) {
      renameSlugPreview = await previewSlug(app.renameTitle);
    } else {
      renameSlugPreview = '';
    }
  }

  // ── UI Actions ────────────────────────────────────────────────────────────────

  function toggleSearch() {
    const nextState = !app.showSearch;
    closeAllDialogs();
    app.showSearch = nextState;
  }

  function toggleNewPage() {
    const nextState = !app.showNewPageForm;
    closeAllDialogs();
    app.showNewPageForm = nextState;
  }

  // ── Delete ────────────────────────────────────────────────────────────────────

  async function handleDelete() {
    if (!app.confirmDeleteSlug) return;
    deleting = true;
    try {
      await deletePage(app.confirmDeleteSlug);
      app.confirmDeleteSlug = null;
    } catch (_) {
      // error set in store
    } finally {
      deleting = false;
    }
  }

  // ── Rename ────────────────────────────────────────────────────────────────────

  async function handleRename() {
    if (!app.renameSlug || !app.renameTitle.trim()) return;
    renaming = true;
    renameError = '';
    try {
      await renamePage(app.renameSlug, app.renameTitle.trim());
      app.renameSlug = null;
      app.renameTitle = '';
    } catch (e) {
      renameError = String(e);
    } finally {
      renaming = false;
    }
  }
</script>

<aside class="page-list">
  <!-- Header -->
  <div class="list-header">
    <span class="list-title">Pages</span>
    <div class="header-actions">
      <button
        class="icon-btn"
        title="Search pages (Ctrl+P)"
        onclick={toggleSearch}
        disabled={app.showExitPrompt}
      >
        <Search size={14} />
      </button>
      <button
        class="icon-btn"
        title="New page (Ctrl+N)"
        onclick={toggleNewPage}
        disabled={app.showSearch || app.showExitPrompt}
      >
        <Plus size={16} />
      </button>
    </div>
  </div>

  <!-- Page items -->
  <ul class="items" bind:this={itemsList} onkeydown={handleListKeydown}>
    {#each app.pages as page}
      <li class="item" class:active={page.slug === app.currentSlug}>
        {#if app.renameSlug === page.slug}
          <!-- Inline rename form -->
          <div class="rename-form">
            <input
              class="input"
              bind:value={app.renameTitle}
              oninput={onRenameTitleInput}
              onkeydown={(e) => {
                if (e.key === 'Enter') handleRename();
              }}
              autofocus
            />
            {#if renameSlugPreview}
              <span class="slug-hint">/{renameSlugPreview}</span>
            {/if}
            {#if renameError}
              <span class="error">{renameError}</span>
            {/if}
            <div class="form-actions">
              <button class="btn-primary" onclick={handleRename} disabled={renaming}>
                {renaming ? '…' : 'Rename'}
              </button>
              <button class="btn-ghost" onclick={() => { app.renameSlug = null; }}>
                <X size={14} />
              </button>
            </div>
          </div>
        {:else}
          <button
            class="page-btn"
            onclick={() => openPage(page.slug)}
          >
            <FileText size={14} class="opacity-70" />
            {page.title}
          </button>
          <div class="page-actions">
            <button
              class="icon-btn small"
              title="Rename"
              onclick={(e) => { e.stopPropagation(); triggerRename(page.slug); }}
            >
              <Pencil size={12} />
            </button>
            <button
              class="icon-btn small danger"
              title="Delete"
              onclick={(e) => { e.stopPropagation(); triggerDelete(page.slug); }}
            >
              <Trash2 size={12} />
            </button>
          </div>
        {/if}
      </li>
    {/each}
    {#if app.pages.length === 0 && !app.loading}
      <li class="empty-hint">No pages yet.</li>
    {/if}
  </ul>
</aside>

<!-- Delete confirmation overlay -->
{#if app.confirmDeleteSlug}
  {@const page = app.pages.find(p => p.slug === app.confirmDeleteSlug)}
  <div class="modal-backdrop" role="presentation" onclick={() => { app.confirmDeleteSlug = null; }}>
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <div class="modal" role="dialog" aria-modal="true" onclick={(e) => e.stopPropagation()}>
      <p class="modal-title">Delete page?</p>
      <p class="modal-body">
        This will permanently delete <strong>{page?.title ?? app.confirmDeleteSlug} [slug:{app.confirmDeleteSlug}]</strong> and all its assets.
      </p>
      <div class="modal-actions">
        <button class="btn-danger" onclick={handleDelete} disabled={deleting}>
          {deleting ? 'Deleting…' : 'Delete'}
        </button>
        <button class="btn-ghost" onclick={() => { app.confirmDeleteSlug = null; }}>Cancel</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .page-list {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
    font-size: 0.875rem;
  }
  .list-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid var(--border);
  }
  .list-title { font-weight: 600; font-size: 0.8125rem; color: var(--muted-foreground); text-transform: uppercase; letter-spacing: 0.05em; }
  .header-actions { display: flex; gap: 0.125rem; }
  .icon-btn {
    display: flex; align-items: center; justify-content: center;
    width: 1.5rem; height: 1.5rem;
    border: none; background: transparent; cursor: pointer; border-radius: 0.25rem;
    font-size: 1rem; color: var(--muted-foreground);
  }
  .icon-btn:hover { background: var(--accent-hover); color: var(--fg); }
  .icon-btn.small { width: 1.25rem; height: 1.25rem; font-size: 0.75rem; opacity: 0; }
  .icon-btn.danger:hover { color: #dc2626; background: #fee2e2; }
  
  .rename-form {
    padding: 0.375rem;
    display: flex; flex-direction: column; gap: 0.25rem;
    width: 100%;
  }
  .input {
    padding: 0.3125rem 0.5rem;
    font-size: 0.8125rem;
    border: 1px solid var(--border);
    border-radius: 0.375rem;
    background: var(--bg);
    color: inherit;
    width: 100%;
    box-sizing: border-box;
  }
  .slug-hint { font-size: 0.6875rem; color: var(--muted-foreground); font-family: monospace; }
  .error { font-size: 0.75rem; color: #dc2626; }
  .form-actions { display: flex; gap: 0.25rem; }
  .btn-primary {
    padding: 0.25rem 0.625rem;
    background: var(--primary); color: #fff;
    border: none; border-radius: 0.25rem; cursor: pointer; font-size: 0.8125rem;
  }
  .btn-primary:disabled { opacity: 0.6; }
  .btn-ghost {
    padding: 0.25rem 0.625rem;
    background: transparent; border: 1px solid var(--border);
    border-radius: 0.25rem; cursor: pointer; font-size: 0.8125rem; color: inherit;
  }
  .btn-danger {
    padding: 0.375rem 0.75rem;
    background: #dc2626; color: #fff;
    border: none; border-radius: 0.375rem; cursor: pointer; font-size: 0.875rem;
  }
  .items {
    list-style: none;
    padding: 0.25rem;
    margin: 0;
    overflow-y: auto;
    flex: 1;
  }
  .item {
    display: flex; align-items: center; gap: 0.25rem;
    border-radius: 0.375rem;
    padding: 0;
    margin-bottom: 0.0625rem;
  }
  .item:hover .icon-btn.small { opacity: 1; }
  .item.active { background: var(--accent); }
  .page-btn {
    flex: 1; text-align: left; padding: 0.4375rem 0.75rem;
    display: flex; align-items: center; gap: 0.5rem;
    border: none; background: transparent; cursor: pointer;
    border-radius: 0.375rem; font-size: 0.875rem; color: inherit;
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
    min-width: 0;
    outline: none;
  }
  .page-actions { display: flex; gap: 0.125rem; padding-right: 0.25rem; flex-shrink: 0; }
  .empty-hint { padding: 0.75rem; color: var(--muted-foreground); font-size: 0.8125rem; }
  .modal-backdrop {
    position: fixed; inset: 0; background: rgba(0,0,0,0.4);
    display: flex; align-items: center; justify-content: center; z-index: 200;
  }
  .modal {
    background: var(--bg);
    border-radius: 0.5rem; padding: 1.5rem;
    max-width: 360px; width: 100%;
    box-shadow: 0 8px 32px rgba(0,0,0,0.2);
    color: var(--fg);
  }
  .modal-title { font-weight: 700; font-size: 1rem; margin: 0 0 0.5rem; }
  .modal-body { font-size: 0.875rem; margin: 0 0 1rem; color: var(--muted-foreground); }
  .modal-actions { display: flex; gap: 0.5rem; justify-content: flex-end; }
</style>
