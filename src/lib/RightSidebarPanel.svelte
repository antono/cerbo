<script lang="ts">
  import { listen } from '@tauri-apps/api/event';
  import { app, loadBacklinks, openPage } from './stores.svelte';
  import { INDEX_READY_EVENT } from './backlinks';
  import SimilarNotesSection from './SimilarNotesSection.svelte';

  /**
   * `stack` (docked and floating panel) shows Backlinks then Similar notes.
   * `tabs` (narrow bottom sheet, design D8) shows only the section `tab`; the
   * sheet's tab bar names it, so the section title is left out.
   */
  let {
    uuid,
    variant = 'stack',
    tab = 'backlinks',
  }: { uuid: string; variant?: 'stack' | 'tabs'; tab?: 'backlinks' | 'similar' } = $props();
  let showBacklinks = $derived(variant === 'stack' || tab === 'backlinks');
  let showSimilar = $derived(variant === 'stack' || tab === 'similar');

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

<aside class="right-sidebar-panel" data-variant={variant}>
  <!-- No header bar and no close button: the toolbar's right panel toggle
       hides the panel (specs `desktop-window-toolbar`, `desktop-window-chrome`).
       Each section titles itself and scrolls with its content. -->
  <div class="panel-content">
    {#if showBacklinks}
    <section aria-labelledby={variant === 'stack' ? 'backlinks-title' : undefined} aria-label={variant === 'tabs' ? 'Backlinks' : undefined}>
    {#if variant === 'stack'}<h3 id="backlinks-title" class="panel-title">Backlinks</h3>{/if}
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
    {/if}

    {#if showSimilar}
      <SimilarNotesSection {uuid} />
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

  .panel-title {
    margin: 0 0 0.5rem;
    font-size: 0.7rem;
    font-weight: 700;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--muted-foreground);
    flex: 1;
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
