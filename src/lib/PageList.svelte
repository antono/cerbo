<script lang="ts">
  /**
   * The sidebar page tree (spec `desktop-page-tree`, `sidebar-navigation`).
   * Rows are the visible flattening of the tree built from `app.pages`, so
   * keyboard focus is a plain index step (design D5).
   */
  import {
    FileText,
    Folder,
    FolderOpen,
    ChevronRight,
    ChevronDown,
    Plus,
    Pencil,
    Trash2,
    Search,
    FolderInput,
    FilePlus,
  } from 'lucide-svelte';
  import { tick } from 'svelte';
  import {
    app,
    openPage,
    deletePage,
    closeAllDialogs,
    triggerRename,
    triggerDelete,
    triggerMove,
    triggerRenameFolder,
    triggerNewPageIn,
    toggleFolder,
    collapseFolder,
    expandFolder,
    movePage,
    visibleRows,
  } from './stores.svelte';
  import { isInputFocused } from './hotkeys';
  import { treeKeyAction, type TreeKey, type TreeRow } from './page-tree';
  import ConfirmationDialog from './ConfirmationDialog.svelte';
  import RowMenu, { type RowMenuItem } from './RowMenu.svelte';

  const rows = $derived(visibleRows());

  let deleting = $state(false);
  let itemsList = $state<HTMLUListElement | null>(null);

  // ── Context menu ─────────────────────────────────────────────────────────────

  let menu = $state<{ x: number; y: number; items: RowMenuItem[] } | null>(null);

  function menuItemsFor(row: TreeRow): RowMenuItem[] {
    if (row.kind === 'folder') {
      const path = row.path;
      return [
        { label: 'New page here', icon: FilePlus, onSelect: () => triggerNewPageIn(path) },
        { label: 'Rename folder', icon: Pencil, onSelect: () => triggerRenameFolder(path) },
      ];
    }
    const uuid = row.uuid;
    return [
      { label: 'Open', icon: FileText, onSelect: () => void openPage(uuid) },
      { label: 'Move to…', icon: FolderInput, onSelect: () => triggerMove(uuid) },
      { label: 'Rename', icon: Pencil, onSelect: () => triggerRename(uuid) },
      { label: 'Delete', icon: Trash2, danger: true, onSelect: () => triggerDelete(uuid) },
    ];
  }

  // A click that lands right after the context menu opened (some WebKit
  // builds report one for the right button) must not act on the row.
  let menuOpenedAt = 0;

  function openMenu(e: MouseEvent, row: TreeRow) {
    e.preventDefault();
    menuOpenedAt = e.timeStamp;
    menu = { x: e.clientX, y: e.clientY, items: menuItemsFor(row) };
  }

  function isRowClick(e: MouseEvent): boolean {
    return e.button === 0 && e.timeStamp - menuOpenedAt > 400;
  }

  // ── Keyboard ─────────────────────────────────────────────────────────────────

  function rowButtons(): HTMLButtonElement[] {
    return Array.from(itemsList?.querySelectorAll<HTMLButtonElement>('.row-btn') ?? []);
  }

  function focusedIndex(): number {
    return rowButtons().indexOf(document.activeElement as HTMLButtonElement);
  }

  const treeKeys: Record<string, TreeKey> = {
    j: 'next',
    J: 'next',
    ArrowDown: 'next',
    k: 'prev',
    K: 'prev',
    ArrowUp: 'prev',
    h: 'left',
    ArrowLeft: 'left',
    l: 'right',
    ArrowRight: 'right',
    Enter: 'enter',
    ' ': 'space',
  };

  function handleListKeydown(e: KeyboardEvent) {
    if (e.altKey || e.ctrlKey || e.metaKey) return;
    if (isInputFocused()) return;

    const index = focusedIndex();
    const row = rows[index];

    // In preview mode the layout owns j/k (scroll), J/K, r and Delete; the
    // tree keys it does not claim still work on a focused row.
    const previewOwned = ['j', 'k', 'J', 'K', 'ArrowDown', 'ArrowUp', 'r', 'Delete', 'Backspace', 'Tab'];
    if (app.editorTab === 'preview' && (previewOwned.includes(e.key) || !row)) return;

    // Row actions are handled here; the layout's window listener must not
    // act on the current page as well.
    if (e.key === 'r') {
      e.preventDefault();
      e.stopPropagation();
      if (row?.kind === 'folder') triggerRenameFolder(row.path);
      else {
        const uuid = row?.kind === 'page' ? row.uuid : app.currentUuid;
        if (uuid) triggerRename(uuid);
      }
      return;
    }
    if (e.key === 'm') {
      e.preventDefault();
      e.stopPropagation();
      if (row?.kind === 'folder') triggerRenameFolder(row.path);
      else {
        const uuid = row?.kind === 'page' ? row.uuid : app.currentUuid;
        if (uuid) triggerMove(uuid);
      }
      return;
    }
    if (e.key === 'Delete' || e.key === 'Backspace') {
      if (row?.kind === 'folder') return;
      e.preventDefault();
      e.stopPropagation();
      const uuid = row?.kind === 'page' ? row.uuid : app.currentUuid;
      if (uuid) triggerDelete(uuid);
      return;
    }

    const key: TreeKey | undefined = e.key === 'Tab' ? (e.shiftKey ? 'prev' : 'next') : treeKeys[e.key];
    if (!key) return;
    const action = treeKeyAction(rows, index, key);
    if (!action) {
      if (e.key === 'Tab') e.preventDefault();
      return;
    }
    e.preventDefault();
    switch (action.kind) {
      case 'focus':
        rowButtons()[action.index]?.focus();
        break;
      case 'collapse':
        collapseFolder(action.path);
        break;
      case 'expand':
        expandFolder(action.path);
        break;
      case 'toggle':
        toggleFolder(action.path);
        break;
      case 'open':
        void openPage(action.uuid);
        break;
    }
  }

  // Keep the active page's row in view when the open page changes (after a
  // search, a link, a restore). Not on collapse/expand: that would pull the
  // list away from the focused row.
  $effect(() => {
    const uuid = app.currentUuid;
    if (!uuid) return;
    void tick().then(() => {
      const el = itemsList?.querySelector(`[data-uuid="${CSS.escape(uuid)}"]`);
      el?.scrollIntoView({ block: 'nearest' });
    });
  });

  // ── Drag and drop (design D8) ────────────────────────────────────────────────

  let dragUuid = $state<string | null>(null);
  let dropTarget = $state<string | null>(null); // folder path, or '' for the root zone

  function dragPath(): string | null {
    return app.pages.find((p) => p.uuid === dragUuid)?.virtualPath ?? null;
  }

  function onDragStart(e: DragEvent, uuid: string) {
    dragUuid = uuid;
    e.dataTransfer?.setData('text/cerbo-uuid', uuid);
    if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move';
  }

  function onDragEnd() {
    dragUuid = null;
    dropTarget = null;
  }

  function canDrop(target: string): boolean {
    if (!dragUuid) return false;
    return (dragPath() ?? '') !== target;
  }

  function onDragOver(e: DragEvent, target: string) {
    if (!canDrop(target)) return;
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = 'move';
    dropTarget = target;
  }

  function onDragLeave(target: string) {
    if (dropTarget === target) dropTarget = null;
  }

  async function onDrop(e: DragEvent, target: string) {
    e.preventDefault();
    const uuid = dragUuid ?? e.dataTransfer?.getData('text/cerbo-uuid') ?? null;
    onDragEnd();
    if (!uuid || !canDropUuid(uuid, target)) return;
    try {
      await movePage(uuid, target === '' ? null : target);
    } catch (err) {
      app.error = String(err);
    }
  }

  function canDropUuid(uuid: string, target: string): boolean {
    const page = app.pages.find((p) => p.uuid === uuid);
    return !!page && (page.virtualPath ?? '') !== target;
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

  async function handleDelete() {
    if (!app.confirmDeleteUuid) return;
    deleting = true;
    try {
      await deletePage(app.confirmDeleteUuid);
      app.confirmDeleteUuid = null;
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
      <button class="icon-btn" title="Search pages (Ctrl+P)" onclick={toggleSearch} disabled={app.showExitPrompt}>
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

  <!-- Tree rows -->
  <div class="items-wrap" onkeydown={handleListKeydown} role="tree" tabindex="0" aria-label="Page tree">
    <ul class="items" bind:this={itemsList}>
      {#each rows as row (row.kind === 'folder' ? `f:${row.path}` : `p:${row.uuid}`)}
        {#if row.kind === 'folder'}
          <li
            class="item folder"
            class:drop-target={dropTarget === row.path}
            role="treeitem"
            aria-selected={false}
            aria-expanded={row.expanded}
            aria-level={row.depth + 1}
            data-kind="folder"
            data-path={row.path}
            ondragover={(e) => onDragOver(e, row.path)}
            ondragleave={() => onDragLeave(row.path)}
            ondrop={(e) => onDrop(e, row.path)}
            oncontextmenu={(e) => openMenu(e, row)}
          >
            <button
              class="row-btn"
              style="padding-left: {0.5 + row.depth * 0.875}rem"
              onclick={(e) => { if (isRowClick(e)) toggleFolder(row.path); }}
              title={row.path}
            >
              <span class="chevron">
                {#if row.expanded}<ChevronDown size={12} />{:else}<ChevronRight size={12} />{/if}
              </span>
              {#if row.expanded}<FolderOpen size={14} class="opacity-70" />{:else}<Folder size={14} class="opacity-70" />{/if}
              <span class="label">{row.name}</span>
            </button>
            <div class="page-actions">
              <button
                class="icon-btn small"
                title="New page here"
                onclick={(e) => { e.stopPropagation(); triggerNewPageIn(row.path); }}
              >
                <FilePlus size={12} />
              </button>
              <button
                class="icon-btn small"
                title="Rename folder"
                onclick={(e) => { e.stopPropagation(); triggerRenameFolder(row.path); }}
              >
                <Pencil size={12} />
              </button>
            </div>
          </li>
        {:else}
          <li
            class="item page"
            class:active={row.uuid === app.currentUuid}
            class:dragging={row.uuid === dragUuid}
            role="treeitem"
            aria-selected={row.uuid === app.currentUuid}
            aria-level={row.depth + 1}
            data-kind="page"
            data-uuid={row.uuid}
            data-path={row.path ?? ''}
            draggable="true"
            ondragstart={(e) => onDragStart(e, row.uuid)}
            ondragend={onDragEnd}
            oncontextmenu={(e) => openMenu(e, row)}
          >
            <button
              class="row-btn page-btn"
              style="padding-left: {0.75 + row.depth * 0.875}rem"
              onclick={(e) => { if (isRowClick(e)) openPage(row.uuid); }}
            >
              <FileText size={14} class="opacity-70" />
              <span class="label">{row.title}</span>
            </button>
            <div class="page-actions">
              <button
                class="icon-btn small"
                title="Move to…"
                onclick={(e) => { e.stopPropagation(); triggerMove(row.uuid); }}
              >
                <FolderInput size={12} />
              </button>
              <button
                class="icon-btn small"
                title="Rename"
                onclick={(e) => { e.stopPropagation(); triggerRename(row.uuid); }}
              >
                <Pencil size={12} />
              </button>
              <button
                class="icon-btn small danger"
                title="Delete"
                onclick={(e) => { e.stopPropagation(); triggerDelete(row.uuid); }}
              >
                <Trash2 size={12} />
              </button>
            </div>
          </li>
        {/if}
      {/each}
      {#if rows.length === 0 && !app.loading}
        <li class="empty-hint">No pages yet.</li>
      {/if}
    </ul>
    <!-- Dropping here moves a page to the root. -->
    <div
      class="root-drop"
      class:visible={dragUuid !== null && dragPath() !== null}
      class:drop-target={dropTarget === ''}
      role="presentation"
      ondragover={(e) => onDragOver(e, '')}
      ondragleave={() => onDragLeave('')}
      ondrop={(e) => onDrop(e, '')}
    >
      {#if dragUuid !== null && dragPath() !== null}<span>Drop here to move to the root</span>{/if}
    </div>
  </div>
</aside>

{#if menu}
  <RowMenu x={menu.x} y={menu.y} items={menu.items} onClose={() => (menu = null)} />
{/if}

<!-- Delete confirmation overlay -->
{#if app.confirmDeleteUuid}
  {@const page = app.pages.find(p => p.uuid === app.confirmDeleteUuid)}
  <ConfirmationDialog
    title="Delete page?"
    message={`This will permanently delete "${page?.title ?? app.confirmDeleteUuid}" and all its assets.`}
    confirmLabel={deleting ? 'Deleting…' : 'Delete'}
    confirmDisabled={deleting}
    onClose={() => { app.confirmDeleteUuid = null; }}
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

  .items-wrap {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    outline: none;
  }
  .items {
    list-style: none;
    padding: 0.25rem;
    margin: 0;
  }
  .item {
    display: flex; align-items: center; gap: 0.25rem;
    border-radius: 0.375rem;
    padding: 0;
    margin-bottom: 0.0625rem;
  }
  .item:hover .icon-btn.small { opacity: 1; }
  .item.active { background: var(--accent); }
  .item.dragging { opacity: 0.5; }
  .item.drop-target { background: var(--accent-hover); box-shadow: inset 0 0 0 1px var(--primary); }
  .row-btn {
    flex: 1; text-align: left; padding: 0.4375rem 0.75rem;
    display: flex; align-items: center; gap: 0.5rem;
    border: none; background: transparent; cursor: pointer;
    border-radius: 0.375rem; font-size: 0.875rem; color: inherit;
    white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
    min-width: 0;
    outline: none;
  }
  .row-btn:focus-visible { box-shadow: inset 0 0 0 1px var(--primary); }
  .row-btn .label { overflow: hidden; text-overflow: ellipsis; }
  .folder .row-btn { color: var(--muted-foreground); font-weight: 500; }
  .folder .row-btn:hover { color: var(--fg); }
  .chevron { display: flex; align-items: center; width: 0.75rem; flex-shrink: 0; }
  .page-actions { display: flex; gap: 0.125rem; padding-right: 0.25rem; flex-shrink: 0; }
  .empty-hint { padding: 0.75rem; color: var(--muted-foreground); font-size: 0.8125rem; }

  .root-drop {
    flex: 1;
    min-height: 1.5rem;
    margin: 0.25rem;
    border-radius: 0.375rem;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 0.75rem;
    color: var(--muted-foreground);
    border: 1px dashed transparent;
  }
  .root-drop.visible { border-color: var(--border); }
  .root-drop.drop-target { border-color: var(--primary); background: var(--accent-hover); }
</style>
