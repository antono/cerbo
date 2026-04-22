<script lang="ts">
  import { Eye, Pencil } from 'lucide-svelte';
  import { app, activeVault } from '$lib/stores.svelte';
  import PageEditor from '$lib/PageEditor.svelte';
  import BacklinksPanel from '$lib/BacklinksPanel.svelte';
  import AttachmentsPanel from '$lib/AttachmentsPanel.svelte';

  // ── State ─────────────────────────────────────────────────────────────────────

  let isSaving = $state(false);
  let isResizingBacklinks = $state(false);

  // ── Derived ───────────────────────────────────────────────────────────────────

  let vault = $derived(activeVault());

  // ── Resizing Logic ──────────────────────────────────────────────────────────

  function startBacklinksResize() {
    isResizingBacklinks = true;
    window.addEventListener('mousemove', handleBacklinksResize);
    window.addEventListener('mouseup', stopResizing);
  }

  function handleBacklinksResize(e: MouseEvent) {
    if (!isResizingBacklinks) return;
    app.backlinksWidth = Math.max(250, Math.min(600, window.innerWidth - e.clientX));
  }

  function stopResizing() {
    isResizingBacklinks = false;
    window.removeEventListener('mousemove', handleBacklinksResize);
    window.removeEventListener('mouseup', stopResizing);
  }
</script>

{#if app.currentSlug}
  <div class="editor-area" class:is-resizing={isResizingBacklinks}>
    <!-- Save indicator (floating or subtle) -->
    {#if isSaving}
      <div class="save-indicator">Saving…</div>
    {/if}

    <!-- Editor -->
    <div class="editor-wrap">
      <PageEditor 
        slug={app.currentSlug} 
        onSaving={(s) => isSaving = s}
      />
      
      {#if !app.backlinksVisible}
        <button 
          class="show-backlinks-btn" 
          onclick={() => app.backlinksVisible = true}
          title="Show panels"
        >
          <Eye size={16} />
        </button>
      {/if}
    </div>

    <!-- Right Resize Handle -->
    {#if app.backlinksVisible}
      <div 
        class="resize-handle" 
        onmousedown={startBacklinksResize}
        role="separator"
        aria-orientation="vertical"
      ></div>

      <!-- Side Panels -->
      <aside class="right-panels" style="width: {app.backlinksWidth}px;">
        <div class="panel-section">
          <BacklinksPanel slug={app.currentSlug} />
        </div>
        <div class="panel-section">
          <AttachmentsPanel slug={app.currentSlug} />
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

  .save-indicator {
    position: absolute;
    top: 0.75rem;
    right: 1.5rem;
    z-index: 50;
    font-size: 0.75rem;
    color: var(--muted-foreground);
    font-style: italic;
    background: var(--bg);
    padding: 2px 8px;
    border-radius: var(--radius);
    border: 1px solid var(--border);
    pointer-events: none;
  }

  .editor-wrap {
    flex: 1;
    min-width: 0;
    height: 100%;
    position: relative;
  }

  .show-backlinks-btn {
    position: absolute;
    top: calc(var(--header-height) + 1rem);
    right: 1.5rem;
    z-index: 30;
    background: var(--bg);
    border: 1px solid var(--border);
    color: var(--muted-foreground);
    padding: 0.5rem;
    border-radius: var(--radius);
    cursor: pointer;
    box-shadow: 0 2px 8px rgba(0,0,0,0.08);
    transition: all 0.15s;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .show-backlinks-btn:hover {
    color: var(--primary);
    border-color: var(--primary);
    background: var(--accent);
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
</style>
