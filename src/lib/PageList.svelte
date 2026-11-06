<script lang="ts">
  /**
   * The sidebar page tree (spec `desktop-page-tree`, `sidebar-navigation`,
   * `desktop-tree-editing`). Rows are the visible flattening of the tree built
   * from `app.pages` and the vault layout, so keyboard focus is a plain index
   * step (design D5). Drag uses mouse events, not native DnD (D8, D7).
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
    FolderPlus,
    FolderMinus,
    ArrowUp,
    ArrowDown,
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
    triggerNewFolder,
    toggleFolder,
    collapseFolder,
    expandFolder,
    pageTree,
    placeRow,
    deleteFolder,
  } from './stores.svelte';
  import { isInputFocused } from './hotkeys';
  import {
    canMoveBy,
    flattenVisible,
    itemOf,
    moveBy,
    treeKeyAction,
    type Placement,
    type TreeItem,
    type TreeKey,
    type TreeRow,
  } from './page-tree';
  import { dropPlacement, dropZone, passedThreshold, rootPlacement, type DropZone } from './tree-drag';
  import ConfirmationDialog from './ConfirmationDialog.svelte';
  import RowMenu, { type RowMenuItem } from './RowMenu.svelte';

  const tree = $derived(pageTree());
  const rows = $derived(flattenVisible(tree, new Set(app.collapsedFolders)));

  let deleting = $state(false);
  let itemsList = $state<HTMLUListElement | null>(null);

  function rowKey(row: TreeRow): string {
    return row.kind === 'folder' ? `f:${row.path}` : `p:${row.uuid}`;
  }

  // ── Placing rows ─────────────────────────────────────────────────────────────

  async function place(p: Placement | null): Promise<boolean> {
    if (!p) return false;
    try {
      await placeRow(p);
      return true;
    } catch (err) {
      app.error = String(err);
      return false;
    }
  }

  function focusRow(key: string) {
    const li = itemsList?.querySelector<HTMLElement>(`[data-key="${CSS.escape(key)}"]`);
    li?.querySelector<HTMLButtonElement>('.row-btn')?.focus();
  }

  async function moveRow(row: TreeRow, delta: -1 | 1) {
    if (await place(moveBy(tree, row, delta))) {
      await tick();
      focusRow(rowKey(row));
    }
  }

  async function removeFolder(path: string) {
    try {
      await deleteFolder(path);
    } catch (err) {
      app.error = String(err);
    }
  }

  // ── Context menu ─────────────────────────────────────────────────────────────

  let menu = $state<{ x: number; y: number; items: RowMenuItem[] } | null>(null);

  function orderItems(row: TreeRow): RowMenuItem[] {
    return [
      { label: 'Move up', icon: ArrowUp, disabled: !canMoveBy(tree, row, -1), onSelect: () => void moveRow(row, -1) },
      { label: 'Move down', icon: ArrowDown, disabled: !canMoveBy(tree, row, 1), onSelect: () => void moveRow(row, 1) },
    ];
  }

  function menuItemsFor(row: TreeRow): RowMenuItem[] {
    if (row.kind === 'folder') {
      const path = row.path;
      const items: RowMenuItem[] = [
        { label: 'New page here', icon: FilePlus, onSelect: () => triggerNewPageIn(path) },
        { label: 'New folder here', icon: FolderPlus, onSelect: () => triggerNewFolder(path) },
        { label: 'Rename folder', icon: Pencil, onSelect: () => triggerRenameFolder(path) },
        ...orderItems(row),
      ];
      if (row.pageCount === 0) {
        items.push({ label: 'Delete folder', icon: FolderMinus, danger: true, onSelect: () => void removeFolder(path) });
      }
      return items;
    }
    const uuid = row.uuid;
    return [
      { label: 'Open', icon: FileText, onSelect: () => void openPage(uuid) },
      { label: 'Move to…', icon: FolderInput, onSelect: () => triggerMove(uuid) },
      { label: 'Rename', icon: Pencil, onSelect: () => triggerRename(uuid) },
      ...orderItems(row),
      { label: 'Delete', icon: Trash2, danger: true, onSelect: () => triggerDelete(uuid) },
    ];
  }

  const addItems: RowMenuItem[] = [
    { label: 'New page', icon: FilePlus, onSelect: () => triggerNewPageIn('') },
    { label: 'New folder', icon: FolderPlus, onSelect: () => triggerNewFolder('') },
  ];

  // A click that lands right after the context menu opened (some WebKit
  // builds report one for the right button) must not act on the row.
  let menuOpenedAt = 0;

  function openMenu(e: MouseEvent, row: TreeRow) {
    e.preventDefault();
    e.stopPropagation();
    menuOpenedAt = e.timeStamp;
    menu = { x: e.clientX, y: e.clientY, items: menuItemsFor(row) };
  }

  /** Right-click on sidebar space that is not a row: the Add menu. */
  function openAddMenu(e: MouseEvent) {
    if ((e.target as Element | null)?.closest('.item')) return;
    e.preventDefault();
    menuOpenedAt = e.timeStamp;
    menu = { x: e.clientX, y: e.clientY, items: addItems };
  }

  function isRowClick(e: MouseEvent): boolean {
    return e.button === 0 && e.timeStamp - menuOpenedAt > 400 && !suppressClick;
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

  const moveKeys: Record<string, -1 | 1> = { ArrowUp: -1, k: -1, K: -1, ArrowDown: 1, j: 1, J: 1 };

  function handleListKeydown(e: KeyboardEvent) {
    if (isInputFocused()) return;

    // Alt+ArrowUp/Down and Alt+k/j move the focused row among its siblings.
    if (e.altKey && !e.ctrlKey && !e.metaKey && e.key in moveKeys) {
      const row = rows[focusedIndex()];
      if (!row) return;
      e.preventDefault();
      e.stopPropagation();
      void moveRow(row, moveKeys[e.key]);
      return;
    }
    if (e.altKey || e.ctrlKey || e.metaKey) return;

    const index = focusedIndex();
    const row = rows[index];

    // This handler only runs while focus is inside the tree, so the tree owns
    // its keys in both editor modes. Every key it handles stops here: the
    // layout's window listener would otherwise also scroll the preview (j/k,
    // arrows) or act on the open page (r, m, Delete).
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
    // A tree key is consumed even when it has nothing to do (h on a root
    // page, k on the first row), so it never falls through to the preview.
    e.preventDefault();
    e.stopPropagation();
    const action = treeKeyAction(rows, index, key);
    if (!action) return;
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

  // ── Pointer drag (add-virtual-path-tree D8, add-sidebar-ordering D7) ─────────
  // Mouse events, not pointer events: WebKit turns a left press into a chorded
  // `pointermove` (no `pointerdown`) while it still counts another button as
  // held, which happens when a context menu swallowed a right-button release.
  // `mousedown` fires for every press.

  interface Drag {
    item: TreeItem;
    key: string;
    label: string;
    x: number;
    y: number;
    /** The row under the pointer and its zone, or 'root' for the root zone. */
    target: { key: string; zone: DropZone } | 'root' | null;
    placement: Placement | null;
  }

  let pending: { row: TreeRow; x: number; y: number } | null = null;
  let drag = $state<Drag | null>(null);
  // The click that ends a drag must not open or toggle the row.
  let suppressClick = false;

  function onRowMouseDown(e: MouseEvent, row: TreeRow) {
    if (e.button !== 0 || (e.target as Element | null)?.closest('.page-actions')) return;
    pending = { row, x: e.clientX, y: e.clientY };
    window.addEventListener('mousemove', onPointerMove, true);
    window.addEventListener('mouseup', onPointerUp, true);
    window.addEventListener('keydown', onDragKeydown, true);
    window.addEventListener('blur', endDrag);
  }

  function hoverTarget(x: number, y: number, d: Drag): Pick<Drag, 'target' | 'placement'> {
    const el = document.elementFromPoint(x, y);
    const li = el?.closest<HTMLElement>('.page-list .item[data-key]');
    if (li) {
      const target = rows.find((r) => rowKey(r) === li.dataset.key);
      if (!target) return { target: null, placement: null };
      const rect = li.getBoundingClientRect();
      const zone = dropZone(target, rect.top, rect.height, y);
      const placement = dropPlacement(tree, d.item, target, zone);
      return placement ? { target: { key: rowKey(target), zone }, placement } : { target: null, placement: null };
    }
    if (el?.closest('.page-list .root-drop')) {
      const placement = rootPlacement(tree, d.item);
      return placement ? { target: 'root', placement } : { target: null, placement: null };
    }
    return { target: null, placement: null };
  }

  function onPointerMove(e: MouseEvent) {
    if (!drag) {
      if (!pending || !passedThreshold(e.clientX - pending.x, e.clientY - pending.y)) return;
      const row = pending.row;
      drag = {
        item: itemOf(row),
        key: rowKey(row),
        label: row.kind === 'folder' ? row.name : row.title,
        x: e.clientX,
        y: e.clientY,
        target: null,
        placement: null,
      };
      document.body.classList.add('tree-dragging');
    }
    e.preventDefault();
    drag.x = e.clientX;
    drag.y = e.clientY;
    Object.assign(drag, hoverTarget(e.clientX, e.clientY, drag));
  }

  function onPointerUp() {
    const finished = drag;
    endDrag();
    if (!finished) return;
    suppressClick = true;
    setTimeout(() => (suppressClick = false), 0);
    void place(finished.placement);
  }

  function onDragKeydown(e: KeyboardEvent) {
    if (e.key !== 'Escape' || !drag) return;
    e.preventDefault();
    e.stopPropagation();
    endDrag();
  }

  function endDrag() {
    pending = null;
    drag = null;
    document.body.classList.remove('tree-dragging');
    window.removeEventListener('mousemove', onPointerMove, true);
    window.removeEventListener('mouseup', onPointerUp, true);
    window.removeEventListener('keydown', onDragKeydown, true);
    window.removeEventListener('blur', endDrag);
  }

  function dropClass(row: TreeRow): string {
    const t = drag?.target;
    if (!t || t === 'root' || t.key !== rowKey(row)) return '';
    return t.zone === 'before' ? 'drop-before' : t.zone === 'after' ? 'drop-after' : 'drop-inside';
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
      <button class="icon-btn" title="New folder" onclick={() => triggerNewFolder('')} disabled={app.showExitPrompt}>
        <FolderPlus size={14} />
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
  <div
    class="items-wrap"
    onkeydown={handleListKeydown}
    oncontextmenu={openAddMenu}
    role="tree"
    tabindex="0"
    aria-label="Page tree"
  >
    <ul class="items" bind:this={itemsList}>
      {#each rows as row (rowKey(row))}
        {#if row.kind === 'folder'}
          <li
            class="item folder {dropClass(row)}"
            class:dragging={drag?.key === rowKey(row)}
            style="--indent: {0.5 + row.depth * 0.875}rem"
            role="treeitem"
            aria-selected={false}
            aria-expanded={row.expanded}
            aria-level={row.depth + 1}
            data-kind="folder"
            data-key={rowKey(row)}
            data-path={row.path}
            onmousedown={(e) => onRowMouseDown(e, row)}
            oncontextmenu={(e) => openMenu(e, row)}
          >
            <button
              class="row-btn"
              style="padding-left: var(--indent)"
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
            class="item page {dropClass(row)}"
            class:active={row.uuid === app.currentUuid}
            class:dragging={drag?.key === rowKey(row)}
            style="--indent: {0.75 + row.depth * 0.875}rem"
            role="treeitem"
            aria-selected={row.uuid === app.currentUuid}
            aria-level={row.depth + 1}
            data-kind="page"
            data-key={rowKey(row)}
            data-uuid={row.uuid}
            data-path={row.path ?? ''}
            onmousedown={(e) => onRowMouseDown(e, row)}
            oncontextmenu={(e) => openMenu(e, row)}
          >
            <button
              class="row-btn page-btn"
              style="padding-left: var(--indent)"
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
    <!-- The space below the rows: dropping here puts the row last at the
         root; right-clicking it opens the Add menu. -->
    <div class="root-drop" class:visible={drag !== null} class:drop-target={drag?.target === 'root'} role="presentation">
      {#if drag}<span>Drop here to move to the root</span>{/if}
    </div>
  </div>
</aside>

{#if drag}
  <div class="drag-ghost" style="left: {drag.x + 12}px; top: {drag.y + 8}px">{drag.label}</div>
{/if}

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
    user-select: none;
    -webkit-user-select: none;
  }
  .items {
    list-style: none;
    padding: 0.25rem;
    margin: 0;
  }
  .item {
    position: relative;
    display: flex; align-items: center; gap: 0.25rem;
    border-radius: 0.375rem;
    padding: 0;
    margin-bottom: 0.0625rem;
  }
  .item:hover .icon-btn.small { opacity: 1; }
  .item.active { background: var(--accent); }
  .item.dragging { opacity: 0.5; }
  .item.drop-inside { background: var(--accent-hover); box-shadow: inset 0 0 0 1px var(--primary); }
  .item.drop-before::before,
  .item.drop-after::after {
    content: '';
    position: absolute;
    left: var(--indent);
    right: 0.25rem;
    height: 2px;
    border-radius: 1px;
    background: var(--primary);
    pointer-events: none;
  }
  .item.drop-before::before { top: -1.5px; }
  .item.drop-after::after { bottom: -1.5px; }
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

  .drag-ghost {
    position: fixed;
    z-index: 950;
    pointer-events: none;
    padding: 0.2rem 0.5rem;
    border-radius: 0.375rem;
    background: var(--bg);
    border: 1px solid var(--border);
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
    font-size: 0.8125rem;
    white-space: nowrap;
  }
  :global(body.tree-dragging),
  :global(body.tree-dragging *) {
    cursor: grabbing !important;
  }
</style>
