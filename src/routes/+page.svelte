<script lang="ts">
  import { app, activeVault } from '$lib/stores.svelte';
  import PageEditor from '$lib/PageEditor.svelte';
  import BacklinksPanel from '$lib/BacklinksPanel.svelte';

  // ── State ─────────────────────────────────────────────────────────────────────

  let isEditing = $state(false);
  let isSaving = $state(false);

  // ── Derived ───────────────────────────────────────────────────────────────────

  let vault = $derived(activeVault());
  let currentPage = $derived(
    app.pages.find((p) => p.slug === app.currentSlug) ?? null,
  );
</script>

{#if app.currentSlug}
  <div class="editor-area">
    <!-- Page title bar -->
    <div class="page-title-bar">
      <div class="title-left">
        <h1 class="page-title">{currentPage?.title ?? app.currentSlug}</h1>
        
        <button 
          class="mode-toggle" 
          onclick={() => isEditing = !isEditing}
          title={isEditing ? "Switch to Preview" : "Switch to Source"}
        >
          {#if isEditing}
            <span class="icon">👁</span>
          {:else}
            <span class="icon">✍</span>
          {/if}
        </button>

        {#if isSaving}
          <div class="save-indicator">Saving…</div>
        {/if}
      </div>
    </div>

    <!-- Editor -->
    <div class="editor-wrap">
      <PageEditor 
        slug={app.currentSlug} 
        bind:isEditing={isEditing}
        onSaving={(s) => isSaving = s}
      />
    </div>

    <!-- Backlinks -->
    <BacklinksPanel slug={app.currentSlug} />
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
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }

  .page-title-bar {
    padding: 0.5rem 1.5rem;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    min-height: 3.5rem;
    display: flex;
    align-items: center;
  }

  .title-left {
    display: flex;
    align-items: center;
    gap: 1rem;
    flex: 1;
    min-width: 0;
  }

  .page-title {
    margin: 0;
    font-size: 1.2rem;
    font-weight: 700;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .mode-toggle {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 2rem;
    height: 2rem;
    padding: 0;
    background: var(--accent);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    cursor: pointer;
    font-size: 0.9rem;
    color: var(--fg);
    transition: all 0.15s;
    flex-shrink: 0;
  }

  .mode-toggle:hover {
    background: var(--accent-hover);
    border-color: var(--primary);
  }

  .save-indicator {
    font-size: 0.75rem;
    color: var(--muted-foreground);
    font-style: italic;
  }

  .editor-wrap {
    flex: 1;
    min-height: 0;
    overflow: hidden;
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
