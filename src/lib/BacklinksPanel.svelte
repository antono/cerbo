<script lang="ts">
  import { app, loadBacklinks, openPage } from './stores.svelte';

  let { slug }: { slug: string } = $props();

  // Reload backlinks when slug changes
  $effect(() => {
    if (slug) {
      loadBacklinks(slug);
    }
  });
</script>

<aside class="backlinks-panel">
  <h3 class="panel-title">Backlinks</h3>

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
</aside>

<style>
  .backlinks-panel {
    padding: 0.75rem 1rem;
    border-top: 1px solid var(--border, #e2e8f0);
    font-size: 0.875rem;
  }

  .panel-title {
    margin: 0 0 0.5rem;
    font-size: 0.75rem;
    font-weight: 600;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--muted-foreground, #888);
  }

  .empty {
    margin: 0;
    color: var(--muted-foreground, #888);
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
    padding: 0.25rem 0.5rem;
    border-radius: 0.25rem;
    cursor: pointer;
    text-align: left;
    width: 100%;
    color: var(--foreground, #0f0f0f);
    font-size: 0.875rem;
    transition: background 0.15s;
  }

  .backlink-item:hover {
    background: var(--accent, #f1f5f9);
  }
</style>
