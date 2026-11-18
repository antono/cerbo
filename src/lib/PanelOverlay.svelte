<script lang="ts">
  /**
   * Panels that float over the editor (spec `desktop-responsive-layout`,
   * design D4, D8): the narrow tree drawer, the medium floating right panel
   * and the narrow bottom sheet. Only `app.overlay` opens them; the docked
   * state in `app.showLeftSidebar` / `app.showRightSidebar` is never touched.
   *
   * Rendered inside `.app-shell` (below the toolbar), so the toolbar toggles
   * stay usable and close their own overlay.
   */
  import { fade, fly } from 'svelte/transition';
  import { ChevronDown } from 'lucide-svelte';
  import cerboIcon from '../favicon.svg';
  import { actionAvailable, actionById, actionLabel } from './actions';
  import { appActions } from './action-host';
  import { activeVault, app, closeOverlay } from './stores.svelte';
  import { controlTitle } from './toolbar';
  import PageList from './PageList.svelte';
  import RightSidebarPanel from './RightSidebarPanel.svelte';

  const vaultSwitch = actionById('vault.switch');
  const vaultAdd = actionById('vault.add');
  const newPage = actionById('page.new');

  let vault = $derived(activeVault());

  let narrow = $derived(app.widthClass === 'narrow');
  let drawerOpen = $derived(narrow && app.overlay === 'tree');
  let floatingOpen = $derived(app.widthClass === 'medium' && app.overlay === 'right' && !!app.currentUuid);
  let sheetOpen = $derived(narrow && app.overlay === 'right' && !!app.currentUuid);
  let anyOpen = $derived(drawerOpen || floatingOpen || sheetOpen);

  function runVault() {
    const action = vault ? vaultSwitch : vaultAdd;
    if (actionAvailable(action, appActions)) action.run(appActions);
  }

  function runNewPage() {
    if (actionAvailable(newPage, appActions)) newPage.run(appActions);
  }

  /**
   * A click outside the overlay closes it and goes no further, so it never
   * edits the page under it (spec "Overlays close predictably").
   */
  function onBackdropPointerDown(e: PointerEvent) {
    e.preventDefault();
    e.stopPropagation();
    closeOverlay();
  }

  // The right panel needs an open page; with none there is nothing to float.
  $effect(() => {
    if (app.overlay === 'right' && !app.currentUuid) closeOverlay();
  });
</script>

{#if anyOpen}
  <div
    class="overlay-backdrop"
    class:dim={narrow}
    data-testid="overlay-backdrop"
    role="presentation"
    onpointerdown={onBackdropPointerDown}
    transition:fade={{ duration: 120 }}
  ></div>
{/if}

{#if drawerOpen}
  <aside
    class="tree-drawer"
    data-testid="tree-drawer"
    aria-label="Page tree"
    style="width: min({app.sidebarWidth}px, calc(100% - 48px));"
    transition:fly={{ x: -280, duration: 160 }}
  >
    <div class="drawer-top">
      <button
        class="drawer-btn vault-btn"
        data-testid="drawer-vault"
        title={controlTitle(vault ? actionLabel(vaultSwitch, appActions) : 'Add vault', (vault ? vaultSwitch : vaultAdd).hotkey)}
        aria-label={vault ? `${actionLabel(vaultSwitch, appActions)}: ${vault.name}` : 'Add vault'}
        onclick={runVault}
      >
        <img src={cerboIcon} alt="" width="18" height="18" class="vault-icon" />
        <span class="vault-name">{vault ? vault.name : 'Add vault'}</span>
        {#if vault}<ChevronDown size={12} />{/if}
      </button>
      <button
        class="drawer-btn icon"
        data-testid="drawer-new-page"
        title={controlTitle(actionLabel(newPage, appActions), newPage.hotkey)}
        aria-label={actionLabel(newPage, appActions)}
        disabled={!actionAvailable(newPage, appActions)}
        onclick={runNewPage}
      >
        <newPage.icon size={16} />
      </button>
    </div>
    <div class="drawer-list">
      {#if vault}
        <PageList />
      {/if}
    </div>
  </aside>
{/if}

{#if floatingOpen && app.currentUuid}
  <aside
    class="right-float"
    data-testid="right-panel-overlay"
    aria-label="Right panel"
    style="width: min({app.backlinksWidth}px, calc(100% - 48px));"
    transition:fly={{ x: 280, duration: 160 }}
  >
    <RightSidebarPanel uuid={app.currentUuid} />
  </aside>
{/if}

{#if sheetOpen && app.currentUuid}
  <section class="right-sheet" data-testid="right-panel-sheet" aria-label="Right panel" transition:fly={{ y: 300, duration: 180 }}>
    <RightSidebarPanel uuid={app.currentUuid} />
  </section>
{/if}

<style>
  .overlay-backdrop {
    position: absolute;
    inset: 0;
    z-index: 60;
    background: transparent;
  }

  .overlay-backdrop.dim {
    background: rgba(0, 0, 0, 0.3);
  }

  .tree-drawer,
  .right-float,
  .right-sheet {
    position: absolute;
    z-index: 70;
    background: var(--sidebar-bg);
    box-shadow: 0 8px 28px rgba(0, 0, 0, 0.25);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .tree-drawer {
    top: 0;
    bottom: 0;
    left: 0;
    border-right: 1px solid var(--border);
  }

  .drawer-top {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.375rem;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .drawer-btn {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    height: 32px;
    padding: 0 0.5rem;
    background: none;
    border: none;
    border-radius: var(--radius);
    color: var(--muted-foreground);
    font: inherit;
    font-size: 0.8125rem;
    cursor: pointer;
  }

  .drawer-btn.icon {
    width: 32px;
    padding: 0;
    justify-content: center;
    flex-shrink: 0;
  }

  .drawer-btn:hover:not(:disabled),
  .drawer-btn:focus-visible {
    background: var(--accent-hover);
    color: var(--fg);
    outline: none;
  }

  .drawer-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .vault-btn {
    flex: 1;
    min-width: 0;
    font-weight: 600;
    color: var(--fg);
  }

  .vault-icon {
    flex-shrink: 0;
    display: block;
  }

  .vault-name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .drawer-list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }

  .right-float {
    top: 0;
    bottom: 0;
    right: 0;
    border-left: 1px solid var(--border);
  }

  .right-sheet {
    left: 0;
    right: 0;
    bottom: 0;
    height: 60%;
    border-top: 1px solid var(--border);
    border-radius: 12px 12px 0 0;
  }
</style>
