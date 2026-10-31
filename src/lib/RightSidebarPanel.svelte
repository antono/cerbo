<script lang="ts">
  import { listen } from '@tauri-apps/api/event';
  import { app, loadBacklinks, openPage, saveUiSettings } from './stores.svelte';
  import { INDEX_READY_EVENT } from './backlinks';
  import SimilarNotesSection from './SimilarNotesSection.svelte';

  let { uuid }: { uuid: string } = $props();

  // Reload backlinks when uuid changes
  $effect(() => {
    if (uuid) {
      loadBacklinks(uuid);
    }
  });

  // A vault's index may still be building when its page opens; load again
  // once the desktop reports it ready.
  $effect(() => {
    const current = uuid;
    const unlisten = listen(INDEX_READY_EVENT, () => {
      if (current) loadBacklinks(current);
    });
    return () => {
      unlisten.then((stop) => stop());
    };
  });
</script>

<aside class="right-sidebar-panel">
  <div class="panel-header">
    <h3 class="panel-title">Backlinks</h3>
    <button
      class="close-btn"
      onclick={async () => {
        app.showRightSidebar = false;
        await saveUiSettings();
      }}
      title="Hide right sidebar"
    >
      ✕
    </button>
  </div>

  <div class="panel-content">
    <!-- Sections: Backlinks (titled by the panel header), then Similar notes. -->
    <section aria-label="Backlinks">
    {#if app.backlinksStatus === 'loading'}
      <p class="empty" aria-busy="true">Loading backlinks…</p>
    {:else if app.backlinksStatus !== 'ready'}
      <p class="notice" role="status">{app.backlinksMessage}</p>
    {:else if app.backlinks.length === 0}
      <p class="empty">No pages link here.</p>
    {:else}
      <ul class="backlink-list">
        {#each app.backlinks as link (link.uuid)}
          <li>
            <button
              class="backlink-item"
              onclick={() => openPage(link.uuid)}
              title={link.title}
            >
              {link.title}
            </button>
          </li>
        {/each}
      </ul>
    {/if}
    </section>

    <SimilarNotesSection {uuid} />
  </div>
</aside>

<style>
  .right-sidebar-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--sidebar-bg);
    overflow: hidden;
  }

  .panel-header {
    height: var(--header-height);
    display: flex;
    align-items: center;
    padding: 0 1rem;
    flex-shrink: 0;
  }

  .panel-title {
    margin: 0;
    font-size: 0.7rem;
    font-weight: 700;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--muted-foreground);
    flex: 1;
  }

  .close-btn {
    background: none;
    border: none;
    color: var(--muted-foreground);
    cursor: pointer;
    font-size: 0.75rem;
    padding: 0.25rem;
    border-radius: var(--radius);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.15s, color 0.15s;
  }

  .close-btn:hover {
    background: var(--accent-hover);
    color: var(--fg);
  }

  .panel-content {
    flex: 1;
    overflow-y: auto;
    padding: 1rem;
  }

  .empty {
    margin: 0;
    color: var(--muted-foreground);
    font-size: 0.8125rem;
    font-style: italic;
  }

  .notice {
    margin: 0;
    color: var(--muted-foreground);
    font-size: 0.8125rem;
  }

  .backlink-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .backlink-item {
    background: none;
    border: none;
    padding: 0.4rem 0.6rem;
    border-radius: var(--radius);
    cursor: pointer;
    text-align: left;
    width: 100%;
    color: var(--fg);
    font-size: 0.8125rem;
    transition: background 0.15s, color 0.15s;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .backlink-item:hover {
    background: var(--accent-hover);
    color: var(--primary);
  }
</style>
