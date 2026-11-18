<script lang="ts">
  /**
   * The one bar at the top of the window (change `unify-window-toolbar`, spec
   * `desktop-window-toolbar`, design D2). Every control runs a registry action
   * (`actions.ts`) through `appActions`, so a click does what its hotkey does.
   *
   * The window has no native frame. `data-tauri-drag-region="deep"` on the bar
   * makes every non-clickable child drag the window and toggle maximize on
   * double-click (Tauri 2.11+); buttons stay clickable and never drag.
   */
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { ChevronDown, Ellipsis, Menu, X } from 'lucide-svelte';
  import cerboIcon from '../favicon.svg';
  import {
    OVERFLOW_MENU,
    PAGE_MENU,
    actionAvailable,
    actionById,
    actionIcon,
    actionLabel,
    hotkeyText,
    type Action,
  } from './actions';
  import { appActions } from './action-host';
  import { activeVault, app, closeAllDialogs, revealFolder } from './stores.svelte';
  import { controlTitle, overflowMenuIds, pathSegments, shouldOpenMenu } from './toolbar';
  import RowMenu, { type RowMenuItem } from './RowMenu.svelte';

  const vaultSwitch = actionById('vault.switch');
  const vaultAdd = actionById('vault.add');
  const panelLeft = actionById('panel.left');
  const panelRight = actionById('panel.right');
  const back = actionById('nav.back');
  const forward = actionById('nav.forward');
  const search = actionById('search.pages');
  const newPage = actionById('page.new');
  const mode = actionById('page.mode');
  const quit = actionById('app.quit');

  let vault = $derived(activeVault());
  let page = $derived(app.pages.find((p) => p.uuid === app.currentUuid));
  let segments = $derived(pathSegments(page?.virtualPath));

  // Layout for the window width (spec `desktop-responsive-layout`). Wide shows
  // every control; medium collapses search and the folder path; narrow keeps
  // ☰, back, the title, search, ⋯ and close.
  let narrow = $derived(app.widthClass === 'narrow');
  let medium = $derived(app.widthClass === 'medium');
  // Pressed state follows what is on screen: docked panel or open overlay.
  let leftShown = $derived(appActions.showLeftSidebar);
  let rightShown = $derived(appActions.showRightSidebar);

  /** The action's current name and its tooltip with the hotkey. */
  function named(action: Action): { label: string; title: string } {
    const label = actionLabel(action, appActions);
    return { label, title: controlTitle(label, action.hotkey) };
  }

  // Names and tooltips follow the state (Hide/Show page tree, Edit/Preview).
  let left = $derived(named(panelLeft));
  let b = $derived(named(back));
  let f = $derived(named(forward));
  let s = $derived(named(search));
  let n = $derived(named(newPage));
  let m = $derived(named(mode));
  let r = $derived(named(panelRight));
  let ModeIcon = $derived(actionIcon(mode, appActions));
  let canBack = $derived(actionAvailable(back, appActions));
  let canForward = $derived(actionAvailable(forward, appActions));
  let canNewPage = $derived(actionAvailable(newPage, appActions));
  let canMode = $derived(actionAvailable(mode, appActions));

  function run(action: Action) {
    if (actionAvailable(action, appActions)) action.run(appActions);
  }

  // ── Menus (page ▾ and ⋯), built on the tree's RowMenu (design D3) ───────────

  // Which menu is open lives in `app.toolbarMenu`, so `closeAllDialogs()` closes
  // it and global hotkeys count it as the one transient element; the position
  // and entries stay here.
  type MenuKind = 'page' | 'more' | 'path';
  let menu = $state<{ kind: MenuKind; x: number; y: number; items: RowMenuItem[] } | null>(null);
  let openMenuKind = $derived(menu && app.toolbarMenu === menu.kind ? menu.kind : null);
  let lastClosed: { kind: string; at: number } | null = null;

  function menuItems(ids: readonly string[]): RowMenuItem[] {
    return ids
      .map((id) => actionById(id))
      .filter((a) => actionAvailable(a, appActions))
      .map((a) => ({
        id: a.id,
        label: actionLabel(a, appActions),
        icon: actionIcon(a, appActions),
        hint: a.hotkey ? hotkeyText(a.hotkey) : undefined,
        danger: a.destructive,
        onSelect: () => a.run(appActions),
      }));
  }

  /** The collapsed path's "…" menu: one entry per folder segment. */
  function segmentItems(): RowMenuItem[] {
    return segments.map((seg) => ({
      id: `segment:${seg.path}`,
      label: seg.path,
      onSelect: () => revealFolder(seg.path),
    }));
  }

  function itemsFor(kind: MenuKind): RowMenuItem[] {
    const items =
      kind === 'path' ? segmentItems() : kind === 'page' ? menuItems(PAGE_MENU) : menuItems(overflowMenuIds(OVERFLOW_MENU, app.widthClass));
    // RowMenu closes before it runs the entry. A menu closed by picking an
    // entry was not closed by a press on its own button, so it may reopen at
    // once (`shouldOpenMenu` only guards that press).
    return items.map((item) => ({
      ...item,
      onSelect: () => {
        lastClosed = null;
        item.onSelect();
      },
    }));
  }

  function openMenu(kind: MenuKind, e: MouseEvent) {
    if (!shouldOpenMenu(kind, lastClosed, performance.now())) return;
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    // One transient element at a time (spec `application-lifecycle`).
    closeAllDialogs();
    menu = {
      kind,
      x: kind === 'more' ? rect.right : rect.left,
      y: rect.bottom + 4,
      items: itemsFor(kind),
    };
    app.toolbarMenu = kind;
  }

  function closeMenu() {
    if (menu) lastClosed = { kind: menu.kind, at: performance.now() };
    menu = null;
    app.toolbarMenu = null;
  }

  // The window outline in app.css applies only to a restored window;
  // `window-maximized` on <html> removes it.
  onMount(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;
    const win = getCurrentWindow();
    const sync = async () => {
      const full = (await win.isMaximized()) || (await win.isFullscreen());
      document.documentElement.classList.toggle('window-maximized', full);
    };
    sync().catch(() => {
      // Not running in Tauri: keep the restored look.
    });
    win
      .onResized(() => void sync().catch(() => {}))
      .then((fn) => (disposed ? fn() : (unlisten = fn)))
      .catch(() => {});
    return () => {
      disposed = true;
      unlisten?.();
    };
  });
</script>

<header class="window-toolbar" class:narrow class:medium data-tauri-drag-region="deep" data-testid="window-toolbar" data-width-class={app.widthClass}>
  <!-- Vault menu: opens the selector, or adds the first vault. In the narrow
       layout it moves into the tree drawer. -->
  {#if narrow}
    <!-- no vault control: the drawer has it -->
  {:else if vault}
    {@const v = named(vaultSwitch)}
    <button class="tb-btn vault-btn" data-testid="toolbar-vault" title={v.title} aria-label="{v.label}: {vault.name}" aria-haspopup="dialog" onclick={() => run(vaultSwitch)}>
      <img src={cerboIcon} alt="" width="18" height="18" class="vault-icon" />
      <span class="vault-name">{vault.name}</span>
      <ChevronDown size={12} />
    </button>
  {:else}
    {@const v = named(vaultAdd)}
    <button class="tb-btn vault-btn" data-testid="toolbar-vault" title={v.title} aria-label={v.label} onclick={() => run(vaultAdd)}>
      <img src={cerboIcon} alt="" width="18" height="18" class="vault-icon" />
      <span class="vault-name">Add vault</span>
    </button>
  {/if}

  <button class="tb-btn icon" data-testid="toolbar-left-panel" title={left.title} aria-label="Page tree" aria-pressed={leftShown} onclick={() => run(panelLeft)}>
    {#if narrow}<Menu size={16} />{:else}<panelLeft.icon size={16} />{/if}
  </button>

  <button class="tb-btn icon" data-testid="toolbar-back" title={b.title} aria-label={b.label} disabled={!canBack} onclick={() => run(back)}>
    <back.icon size={16} />
  </button>
  {#if !narrow}
    <button class="tb-btn icon" data-testid="toolbar-forward" title={f.title} aria-label={f.label} disabled={!canForward} onclick={() => run(forward)}>
      <forward.icon size={16} />
    </button>
  {/if}

  <!-- Path of the open page: folder segments reveal their folder, the title
       opens the page menu. -->
  {#if page}
    <nav class="page-path" aria-label="Page path" data-testid="toolbar-path">
      {#if narrow}
        <!-- narrow: the title alone -->
      {:else if medium}
        {#if segments.length}
          <!-- Folder segments collapse to "…", which lists them. -->
          <button
            class="tb-btn segment collapsed"
            data-testid="toolbar-path-collapsed"
            title="Folders: {segments.map((seg) => seg.name).join(' / ')}"
            aria-label="Show folders"
            aria-haspopup="menu"
            aria-expanded={openMenuKind === 'path'}
            onclick={(e) => openMenu('path', e)}
          >…</button>
          <span class="path-sep" aria-hidden="true">/</span>
        {/if}
      {:else}
        {#each segments as seg (seg.path)}
          <button class="tb-btn segment" data-testid="toolbar-segment" data-path={seg.path} title="Reveal {seg.path} in the tree" onclick={() => revealFolder(seg.path)}>{seg.name}</button>
          <span class="path-sep" aria-hidden="true">/</span>
        {/each}
      {/if}
      <button
        class="tb-btn page-title"
        data-testid="toolbar-page-menu"
        title="Page menu"
        aria-label="Page menu: {page.title}"
        aria-haspopup="menu"
        aria-expanded={openMenuKind === 'page'}
        onclick={(e) => openMenu('page', e)}
      >
        <span class="title-text">{page.title}</span>
        <ChevronDown size={12} />
      </button>
      {#if app.pageUnsaved}
        <span class="save-dot" data-testid="toolbar-save-dot" role="status" title="Unsaved changes" aria-label="Unsaved changes"></span>
      {/if}
    </nav>
  {/if}

  <div class="spacer"></div>

  {#if narrow || medium}
    <!-- Search shrinks to an icon below the wide layout. -->
    <button class="tb-btn icon" data-testid="toolbar-search" title={s.title} aria-label={s.label} onclick={() => appActions.openSearch('')}>
      <search.icon size={16} />
    </button>
  {:else}
    <button class="tb-btn search-field" data-testid="toolbar-search" title={s.title} aria-label={s.label} onclick={() => appActions.openSearch('')}>
      <search.icon size={14} />
      <span class="search-placeholder">Search pages or &gt; command</span>
      {#if search.hotkey}<kbd class="search-kbd">{hotkeyText(search.hotkey)}</kbd>{/if}
    </button>
  {/if}

  <!-- Narrow: new page is in the drawer, the mode toggle floats over the
       editor and the right panel toggle is in ⋯. -->
  {#if !narrow}
    <button class="tb-btn icon" data-testid="toolbar-new-page" title={n.title} aria-label={n.label} disabled={!canNewPage} onclick={() => run(newPage)}>
      <newPage.icon size={16} />
    </button>

    <button class="tb-btn icon" data-testid="toolbar-mode" data-mode={app.editorTab} title={m.title} aria-label={m.label} disabled={!canMode} onclick={() => run(mode)}>
      <ModeIcon size={16} />
    </button>

    <button class="tb-btn icon" data-testid="toolbar-right-panel" title={r.title} aria-label="Right panel" aria-pressed={rightShown} onclick={() => run(panelRight)}>
      <panelRight.icon size={16} />
    </button>
  {/if}

  <button
    class="tb-btn icon"
    data-testid="toolbar-more"
    title="More actions"
    aria-label="More actions"
    aria-haspopup="menu"
    aria-expanded={openMenuKind === 'more'}
    onclick={(e) => openMenu('more', e)}
  >
    <Ellipsis size={16} />
  </button>

  <span class="close-sep" aria-hidden="true"></span>

  <button class="tb-close" data-testid="toolbar-close" title={controlTitle('Quit Cerbo', quit.hotkey)} aria-label="Quit Cerbo" onclick={() => run(quit)}>
    <X size={16} />
  </button>
</header>

{#if menu && openMenuKind}
  <RowMenu x={menu.x} y={menu.y} items={menu.items} label={menu.kind === 'page' ? 'Page menu' : menu.kind === 'path' ? 'Folders' : 'More actions'} onClose={closeMenu} />
{/if}

<style>
  .window-toolbar {
    display: flex;
    align-items: center;
    gap: 0.125rem;
    height: var(--titlebar-height);
    flex-shrink: 0;
    padding-left: 0.375rem;
    background: var(--sidebar-bg);
    border-bottom: 1px solid var(--border);
    user-select: none;
    -webkit-user-select: none;
    font-size: 0.8125rem;
    color: var(--muted-foreground);
    overflow: hidden;
  }

  .tb-btn {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    height: 28px;
    padding: 0 0.4rem;
    flex-shrink: 0;
    background: none;
    border: none;
    border-radius: var(--radius);
    color: var(--muted-foreground);
    font: inherit;
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }

  .tb-btn.icon {
    width: 28px;
    padding: 0;
    justify-content: center;
  }

  .tb-btn:hover:not(:disabled),
  .tb-btn:focus-visible {
    background: var(--accent-hover);
    color: var(--fg);
    outline: none;
  }

  .tb-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .tb-btn[aria-pressed='true'] {
    background: var(--accent);
    color: var(--fg);
  }

  .vault-btn {
    max-width: 14rem;
    font-weight: 600;
    color: var(--fg);
  }

  .vault-icon {
    flex-shrink: 0;
    display: block;
  }

  .vault-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .page-path {
    display: flex;
    align-items: center;
    min-width: 0;
    flex: 0 1 auto;
    margin-left: 0.25rem;
    white-space: nowrap;
  }

  .segment {
    flex-shrink: 1;
    min-width: 2rem;
    max-width: 10rem;
    overflow: hidden;
    text-overflow: ellipsis;
    display: block;
    line-height: 28px;
  }

  .path-sep {
    flex-shrink: 0;
    padding: 0 0.1rem;
    opacity: 0.6;
  }

  .page-title {
    min-width: 3rem;
    flex-shrink: 1;
    color: var(--fg);
    font-weight: 500;
  }

  .title-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .save-dot {
    flex-shrink: 0;
    width: 6px;
    height: 6px;
    margin-left: 0.25rem;
    border-radius: 50%;
    background: var(--primary);
  }

  .spacer {
    flex: 1;
    min-width: 0.5rem;
    align-self: stretch;
  }

  /* The search field shrinks first when the bar gets crowded (design risks). */
  .search-field {
    flex: 0 1 18rem;
    min-width: 8rem;
    height: 24px;
    padding: 0 0.5rem;
    border: 1px solid var(--border);
    background: var(--bg);
    overflow: hidden;
  }

  .search-placeholder {
    flex: 1;
    min-width: 0;
    text-align: left;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .search-kbd {
    flex-shrink: 0;
    font-family: inherit;
    font-size: 0.7rem;
    opacity: 0.8;
  }

  /* Narrow layout: touch-sized targets (spec "Narrow layout": at least 32px). */
  .window-toolbar.narrow .tb-btn {
    height: 32px;
  }

  .window-toolbar.narrow .tb-btn.icon {
    width: 32px;
  }

  .window-toolbar.narrow .page-title {
    min-width: 0;
  }

  .window-toolbar.narrow .tb-close {
    width: 36px;
  }

  .close-sep {
    flex-shrink: 0;
    width: 1px;
    height: 18px;
    margin: 0 0.25rem;
    background: var(--border);
  }

  .tb-close {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 44px;
    height: 100%;
    flex-shrink: 0;
    padding: 0;
    background: none;
    border: none;
    cursor: pointer;
    color: var(--muted-foreground);
    transition: background 0.15s, color 0.15s;
  }

  .tb-close:hover,
  .tb-close:focus-visible {
    background: #ef4444;
    color: #fff;
    outline: none;
  }
</style>
