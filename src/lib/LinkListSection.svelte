<script lang="ts">
  /**
   * One titled list of pages in the Links sub-panel (Backlinks, Outgoing
   * links): a loading line, a message, an empty line or the list. A click opens
   * the page by UUID.
   */
  import type { BacklinksStatus } from './backlinks';
  import { openPage, type BacklinkEntry } from './stores.svelte';

  let {
    id,
    title,
    entries,
    status,
    message,
    loadingText,
    emptyText,
  }: {
    id: string;
    title: string;
    entries: BacklinkEntry[];
    status: BacklinksStatus;
    message: string;
    loadingText: string;
    emptyText: string;
  } = $props();
</script>

<section class="link-section" aria-labelledby="{id}-title" data-testid="{id}-section">
  <h3 id="{id}-title" class="section-title">{title}</h3>
  {#if status === 'loading'}
    <p class="empty" aria-busy="true">{loadingText}</p>
  {:else if status !== 'ready'}
    <p class="notice" role="status">{message}</p>
  {:else if entries.length === 0}
    <p class="empty">{emptyText}</p>
  {:else}
    <ul class="link-list">
      {#each entries as link (link.uuid)}
        <li>
          <button class="link-item" onclick={() => openPage(link.uuid)} title={link.title}>
            {link.title}
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .link-section {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .section-title {
    margin: 0;
    font-size: 0.7rem;
    font-weight: 700;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: var(--muted-foreground);
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

  .link-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .link-item {
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

  .link-item:hover,
  .link-item:focus-visible {
    background: var(--accent-hover);
    color: var(--primary);
    outline: none;
  }
</style>
