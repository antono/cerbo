<script lang="ts">
  import { FileText, Plus, Pencil, Trash2, Search } from 'lucide-svelte';
  import { tick } from 'svelte';
  import { app, openPage, createPage, deletePage, renamePage, previewSlug, closeAllDialogs, triggerRename, triggerDelete, openNextPage, openPrevPage } from './stores.svelte';
  import { isInputFocused } from './hotkeys';
  import ConfirmationDialog from './ConfirmationDialog.svelte';

  // ── State for dialogs ─────────────────────────────────────────────────────────

  let deleting = $state(false);

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
  <div class="items-wrap" onkeydown={handleListKeydown} role="listbox" tabindex="0" aria-label="Page list navigation">
    <ul class="items" bind:this={itemsList}>
    {#each app.pages as page (page.slug)}
      <li class="item" class:active={page.slug === app.currentSlug}>
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
      </li>
    {/each}
    {#if app.pages.length === 0 && !app.loading}
      <li class="empty-hint">No pages yet.</li>
    {/if}
    </ul>
  </div>
</aside>

<!-- Delete confirmation overlay -->
{#if app.confirmDeleteSlug}
  {@const page = app.pages.find(p => p.slug === app.confirmDeleteSlug)}
  <ConfirmationDialog
    title="Delete page?"
    message={`This will permanently delete ${page?.title ?? app.confirmDeleteSlug} [slug:${app.confirmDeleteSlug}] and all its assets.`}
    confirmLabel={deleting ? 'Deleting…' : 'Delete'}
    confirmDisabled={deleting}
    onClose={() => { app.confirmDeleteSlug = null; }}
    onConfirm={handleDelete}
  />
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
  
  .items {
    list-style: none;
    padding: 0.25rem;
    margin: 0;
    overflow-y: auto;
    flex: 1;
  }
  .items-wrap {
    flex: 1;
    min-height: 0;
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
</style>
