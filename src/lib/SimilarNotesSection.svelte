<script lang="ts">
  import { untrack } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import {
    app,
    applySemanticProgress,
    enableEmbeddings,
    loadSimilarNotes,
    openPage,
  } from './stores.svelte';
  import {
    PROGRESS_EVENT,
    progressFraction,
    progressLabel,
    scorePercent,
    type SemanticProgress,
  } from './similar-notes';
  import { INDEX_READY_EVENT } from './backlinks';

  let { uuid }: { uuid: string } = $props();

  // Reload when another page opens; answers for a page left behind are
  // dropped by the store (design D2). Only `uuid` is a dependency: the loader
  // reads and replaces `app.similar`, which would otherwise re-run this effect
  // on every request it makes.
  $effect(() => {
    const current = uuid;
    if (current) {
      untrack(() => loadSimilarNotes(current));
    }
  });

  // Model download and embedding steps, from `embeddings_refresh` or from a
  // `similar_get` that has to re-embed changed pages first.
  $effect(() => {
    const unlisten = listen<SemanticProgress>(PROGRESS_EVENT, (event) => {
      applySemanticProgress(event.payload);
    });
    return () => {
      unlisten.then((stop) => stop());
    };
  });

  // The index may have been held by another process; try again once the
  // desktop reports it ready (again), or when the user clicks "Retry".
  $effect(() => {
    const current = uuid;
    const unlisten = listen(INDEX_READY_EVENT, () => {
      if (current && app.similar.status === 'busy') loadSimilarNotes(current);
    });
    return () => {
      unlisten.then((stop) => stop());
    };
  });

  const fraction = $derived(progressFraction(app.similar.progress));
</script>

<section class="similar-section" aria-labelledby="similar-notes-title">
  <h3 id="similar-notes-title" class="section-title">Similar notes</h3>

  {#if app.similar.status === 'loading' || app.similar.status === 'enabling'}
    <div class="progress" role="status" aria-busy="true">
      <p class="empty">
        {app.similar.status === 'enabling' || app.similar.progress
          ? progressLabel(app.similar.progress)
          : 'Loading similar notes…'}
      </p>
      {#if app.similar.status === 'enabling' || app.similar.progress}
        <div
          class="progress-track"
          role="progressbar"
          aria-label="Semantic search progress"
          aria-valuemin="0"
          aria-valuemax="100"
          aria-valuenow={fraction === null ? undefined : Math.round(fraction * 100)}
        >
          <div
            class="progress-bar"
            class:indeterminate={fraction === null}
            style:width={fraction === null ? undefined : `${fraction * 100}%`}
          ></div>
        </div>
      {/if}
    </div>
  {:else if app.similar.status === 'disabled'}
    <div class="notice-block">
      <p class="notice">
        Semantic search is off for this vault. Enable it to compute embeddings and see notes
        close in meaning. The first time, this downloads the embedding model.
      </p>
      <button class="action-btn" onclick={() => enableEmbeddings()}>Enable</button>
    </div>
  {:else if app.similar.status === 'modelUnavailable'}
    <div class="notice-block">
      <p class="notice" role="status">
        The embedding model is unavailable.{app.similar.message ? ` ${app.similar.message}` : ''}
      </p>
      <button class="action-btn" onclick={() => enableEmbeddings()}>Retry</button>
    </div>
  {:else if app.similar.status === 'busy'}
    <div class="notice-block">
      <p class="notice" role="status">{app.similar.message}</p>
      <button class="action-btn" onclick={() => loadSimilarNotes(app.similar.uuid ?? uuid)}>Retry</button>
    </div>
  {:else if app.similar.status === 'error'}
    <p class="notice" role="status">{app.similar.message}</p>
  {:else if app.similar.status === 'ready' && app.similar.notes.length === 0}
    <p class="empty">No similar notes yet.</p>
  {:else if app.similar.status === 'ready'}
    <ul class="similar-list">
      {#each app.similar.notes as note (note.uuid)}
        <li>
          <button
            class="similar-item"
            onclick={() => openPage(note.uuid)}
            title={`${note.title} (${scorePercent(note.score)} similar)`}
          >
            <span class="similar-title">{note.title}</span>
            <span class="similar-score">{scorePercent(note.score)}</span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .similar-section {
    margin-top: 1.5rem;
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

  .notice-block,
  .progress {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    align-items: flex-start;
  }

  .action-btn {
    background: none;
    border: 1px solid var(--border);
    color: var(--fg);
    border-radius: var(--radius);
    padding: 0.3rem 0.75rem;
    font-size: 0.8125rem;
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }

  .action-btn:hover,
  .action-btn:focus-visible {
    background: var(--accent-hover);
    color: var(--primary);
  }

  .progress-track {
    width: 100%;
    height: 4px;
    border-radius: 2px;
    background: var(--border);
    overflow: hidden;
  }

  .progress-bar {
    height: 100%;
    background: var(--primary);
    transition: width 0.2s;
  }

  /* No count yet (e.g. linking similar pages): a sliding segment. */
  .progress-bar.indeterminate {
    width: 30%;
    animation: slide 1.2s ease-in-out infinite;
  }

  @keyframes slide {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(340%);
    }
  }

  .similar-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .similar-item {
    background: none;
    border: none;
    padding: 0.4rem 0.6rem;
    border-radius: var(--radius);
    cursor: pointer;
    text-align: left;
    width: 100%;
    color: var(--fg);
    font-size: 0.8125rem;
    display: flex;
    align-items: baseline;
    gap: 0.5rem;
    transition: background 0.15s, color 0.15s;
  }

  .similar-item:hover {
    background: var(--accent-hover);
    color: var(--primary);
  }

  .similar-title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .similar-score {
    flex-shrink: 0;
    color: var(--muted-foreground);
    font-size: 0.75rem;
    font-variant-numeric: tabular-nums;
  }
</style>
