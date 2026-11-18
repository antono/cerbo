<script lang="ts">
  import { app, activeVault, quickAddVault, dockedPanelWidths } from '#lib/stores.svelte.js';
  import PageEditor from '#lib/PageEditor.svelte';
  import RightSidebarPanel from '#lib/RightSidebarPanel.svelte';
  import AttachmentsPanel from '#lib/AttachmentsPanel.svelte';
  import { actionAvailable, actionById, actionIcon, actionLabel } from '#lib/actions.js';
  import { appActions } from '#lib/action-host.js';
  import { maxPanelWidth } from '#lib/layout.js';
  import { controlTitle } from '#lib/toolbar.js';

  // The edit/preview toggle floats over the editor in the narrow layout
  // (spec `desktop-responsive-layout`, "Narrow layout").
  const modeAction = actionById('page.mode');
  let modeLabel = $derived(actionLabel(modeAction, appActions));
  let ModeIcon = $derived(actionIcon(modeAction, appActions));

  function toggleMode() {
    if (actionAvailable(modeAction, appActions)) modeAction.run(appActions);
  }

  // ── State ─────────────────────────────────────────────────────────────────────

  let isResizingBacklinks = $state(false);

  // ── Derived ───────────────────────────────────────────────────────────────────

  let vault = $derived(activeVault());
  // Drawn width of the docked right panel: re-clamped whenever the window
  // resizes, so the editor keeps 360px (spec "Editor keeps its width").
  let rightWidth = $derived(dockedPanelWidths().right);

  // ── Resizing Logic ──────────────────────────────────────────────────────────

  function startBacklinksResize() {
    isResizingBacklinks = true;
    window.addEventListener('mousemove', handleBacklinksResize);
    window.addEventListener('mouseup', stopResizing);
  }

  function handleBacklinksResize(e: MouseEvent) {
    if (!isResizingBacklinks) return;
    // The editor keeps at least 360px (spec "Editor keeps its width").
    const left = dockedPanelWidths().tree;
    app.backlinksWidth = Math.max(250, Math.min(maxPanelWidth(window.innerWidth, left, 250, 600), window.innerWidth - e.clientX));
  }

  function stopResizing() {
    isResizingBacklinks = false;
    window.removeEventListener('mousemove', handleBacklinksResize);
    window.removeEventListener('mouseup', stopResizing);
  }
</script>

{#if app.currentUuid}
  <div class="editor-area" class:is-resizing={isResizingBacklinks}>
    <!-- Save state is the dot after the title in the window toolbar. -->
    <!-- Editor -->
    <div class="editor-wrap">
      <PageEditor uuid={app.currentUuid} />
      {#if app.widthClass === 'narrow'}
        <button
          class="floating-mode"
          data-testid="floating-mode"
          data-mode={app.editorTab}
          title={controlTitle(modeLabel, modeAction.hotkey)}
          aria-label={modeLabel}
          onclick={toggleMode}
        >
          <ModeIcon size={18} />
        </button>
      {/if}
    </div>

    <!-- Right Resize Handle. Docked only in the wide layout; below it the
         panel floats over the editor (PanelOverlay). -->
    {#if app.showRightSidebar && app.widthClass === 'wide'}
      <div 
        class="resize-handle" 
        onmousedown={startBacklinksResize}
        role="separator"
        aria-orientation="vertical"
      ></div>

      <!-- Side Panels -->
      <aside 
        class="right-panels" 
        style="width: {rightWidth}px;"
        tabindex="-1"
      >
        <div class="panel-section">
          <RightSidebarPanel uuid={app.currentUuid} />
        </div>
        <div class="panel-section">
          <AttachmentsPanel uuid={app.currentUuid} />
        </div>
      </aside>
    {/if}
  </div>
{:else if vault}
  <div class="empty-state">
    <p>Select a page from the sidebar, or create a new one.</p>
  </div>
{:else}
  <div class="empty-state">
    <p>Welcome to Cerbo. Add a vault to begin.</p>
    <button class="add-vault-btn" onclick={quickAddVault}>
      Add vault to begin
    </button>
  </div>
{/if}

<style>
  .editor-area {
    display: flex;
    flex-direction: row; /* Horizontal layout */
    height: 100%;
    min-height: 0;
    width: 100%;
    overflow: hidden;
  }

  .editor-wrap {
    flex: 1;
    min-width: 0;
    height: 100%;
    position: relative;
  }

  .floating-mode {
    position: absolute;
    right: 1rem;
    bottom: 1rem;
    z-index: 30;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 44px;
    height: 44px;
    border: 1px solid var(--border);
    border-radius: 50%;
    background: var(--bg);
    color: var(--fg);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2);
    cursor: pointer;
  }

  .floating-mode:hover,
  .floating-mode:focus-visible {
    background: var(--accent-hover);
    outline: none;
  }

  .resize-handle {
    width: 1px;
    background: var(--border);
    cursor: col-resize;
    transition: background 0.15s, width 0.15s;
    position: relative;
    z-index: 40;
    flex-shrink: 0;
  }

  .resize-handle:hover, .is-resizing .resize-handle {
    background: var(--primary);
    width: 2px;
  }

  /* Transparent wider target area for the resize handle */
  .resize-handle::after {
    content: '';
    position: absolute;
    top: 0;
    bottom: 0;
    left: -4px;
    right: -4px;
  }

  .right-panels {
    height: 100%;
    background: var(--sidebar-bg);
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    border-left: none;
  }

  .panel-section {
    flex: 1;
    min-height: 0;
    border-bottom: 1px solid var(--border);
  }

  .panel-section:last-child {
    border-bottom: none;
  }

  .is-resizing {
    cursor: col-resize;
    user-select: none;
  }

  .empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    color: var(--muted-foreground);
    gap: 1rem;
    font-size: 0.95rem;
  }

  .add-vault-btn {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.625rem 1.25rem;
    background: var(--primary);
    color: #fff;
    border: none;
    border-radius: var(--radius);
    font-size: 0.9rem;
    font-weight: 600;
    cursor: pointer;
    transition: background 0.15s, transform 0.1s;
    box-shadow: 0 2px 4px rgba(0,0,0,0.15);
  }

  .add-vault-btn:hover {
    background: var(--primary-hover);
  }

  .add-vault-btn:active {
    transform: translateY(1px);
  }
</style>
