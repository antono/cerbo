<script lang="ts">
  import { app, loadBacklinks, openPage, saveUiSettings } from './stores.svelte';

  let { slug }: { slug: string } = $props();

  // Reload backlinks when slug changes
  $effect(() => {
    if (slug) {
      loadBacklinks(slug);
    }
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
    {#if app.backlinks.length === 0}
      <p class="empty">No pages link here.</p>
    {:else}
      <ul class="backlink-list">
        {#each app.backlinks as link (link.slug)}
          <li>
            <button
              class="backlink-item"
              onclick={() => openPage(link.slug)}
              title={link.slug}
            >
              {link.title}
            </button>
          </li>
        {/each}
      </ul>
    {/if}
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
