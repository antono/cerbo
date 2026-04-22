<script lang="ts">
  import { Eye, Pencil } from 'lucide-svelte';
  import { app, activeVault } from '$lib/stores.svelte';
  import PageEditor from '$lib/PageEditor.svelte';
  import BacklinksPanel from '$lib/BacklinksPanel.svelte';

  // ── State ─────────────────────────────────────────────────────────────────────

  let isSaving = $state(false);

  // ── Derived ───────────────────────────────────────────────────────────────────

  let vault = $derived(activeVault());
</script>

{#if app.currentSlug}
  <div class="editor-area">
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
