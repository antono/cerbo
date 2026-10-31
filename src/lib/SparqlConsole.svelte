<script lang="ts">
  import { onMount } from 'svelte';
  import { fade, fly } from 'svelte/transition';
  import { Play } from 'lucide-svelte';
  import { app, openPage, runSparqlQuery } from './stores.svelte';
  import { isMac, isModKey, isSparqlConsoleKey } from './hotkeys';
  import { EXAMPLE_QUERY, errorMessage, mapResponse, rowCountLabel, type ConsoleResult } from './sparql-console';

  let { onClose }: { onClose: () => void } = $props();

  const mod = isMac ? '⌘' : 'Ctrl';

  // The last query run in this app, or an example on first open (spec).
  let query = $state(app.lastSparqlQuery ?? EXAMPLE_QUERY);
  let running = $state(false);
  let result = $state<ConsoleResult | null>(null);
  let error = $state<string | null>(null);
  let inputEl = $state<HTMLTextAreaElement | null>(null);

  async function run() {
    if (running || !query.trim()) return;
    running = true;
    error = null;
    try {
      const response = await runSparqlQuery(query);
      result = mapResponse(response, app.pages, app.prefixMap);
    } catch (e) {
      result = null;
      error = errorMessage(e);
    } finally {
      running = false;
    }
  }

  function openResultPage(uuid: string) {
    onClose();
    void openPage(uuid);
  }

  onMount(() => {
    inputEl?.focus();

    function handleWindowKeydown(e: KeyboardEvent) {
      // The console owns the keyboard: nothing leaks to the editor or the
      // app shortcuts (typing still reaches the query field).
      e.stopPropagation();

      if (e.key === 'Escape' || isSparqlConsoleKey(e)) {
        e.preventDefault();
        onClose();
        return;
      }
      if (isModKey(e, 'Enter')) {
        e.preventDefault();
        void run();
      }
    }

    window.addEventListener('keydown', handleWindowKeydown, true);
    return () => window.removeEventListener('keydown', handleWindowKeydown, true);
  });
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="modal-backdrop"
  onclick={(e) => e.target === e.currentTarget && onClose()}
  transition:fade={{ duration: 150 }}
>
  <div
    class="console-modal"
    role="dialog"
    aria-modal="true"
    aria-labelledby="sparql-console-title"
    tabindex="-1"
    transition:fly={{ y: 20, duration: 200 }}
  >
    <header class="modal-header">
      <h2 id="sparql-console-title">SPARQL Console</h2>
      <button class="close-btn" onclick={onClose} title="Close (Esc)" aria-label="Close">✕</button>
    </header>

    <div class="query-wrap">
      <textarea
        bind:this={inputEl}
        bind:value={query}
        class="query-input"
        rows="6"
        spellcheck="false"
        autocomplete="off"
        aria-label="SPARQL query"
      ></textarea>
    </div>

    <div class="toolbar">
      <button class="run-btn" onclick={run} disabled={running || !query.trim()}>
        <Play size={14} />
        <span>{running ? 'Running…' : 'Run'}</span>
      </button>
      <span class="hint"><kbd>{mod} + ↵</kbd> to run · read-only, prefixes predeclared</span>
      <span class="status" aria-live="polite">
        {#if result?.kind === 'table'}
          {rowCountLabel(result.rows.length)}
        {/if}
      </span>
    </div>

    <div class="results">
      {#if error}
        <pre class="error" role="alert">{error}</pre>
      {:else if result}
        {#if 'truncated' in result && result.truncated}
          <p class="notice">The result was truncated: only the first {result.limit} {result.kind === 'table' ? 'rows' : 'triples'} are shown.</p>
        {/if}

        {#if result.kind === 'table'}
          {#if result.rows.length === 0}
            <p class="empty">No results.</p>
          {:else}
            <table>
              <thead>
                <tr>
                  {#each result.vars as v (v)}
                    <th scope="col">{v}</th>
                  {/each}
                </tr>
              </thead>
              <tbody>
                {#each result.rows as row, r (r)}
                  <tr>
                    {#each row as cell, c (c)}
                      <td>
                        {#if cell.kind === 'page'}
                          <button class="page-link" title={cell.iri} onclick={() => openResultPage(cell.uuid)}>{cell.title}</button>
                        {:else if cell.kind === 'iri'}
                          <span class="iri" title={cell.iri}>{cell.text}</span>
                        {:else if cell.kind === 'literal'}
                          <span class="literal">{cell.text}</span>{#if cell.hint}<span class="literal-hint">{cell.hint.startsWith('@') ? cell.hint : `^^${cell.hint}`}</span>{/if}
                        {:else if cell.kind === 'bnode'}
                          <span class="bnode">{cell.text}</span>
                        {/if}
                      </td>
                    {/each}
                  </tr>
                {/each}
              </tbody>
            </table>
          {/if}
        {:else if result.kind === 'boolean'}
          <p class="boolean">{result.value ? 'true' : 'false'}</p>
        {:else}
          <pre class="turtle">{result.text || '# Empty graph'}</pre>
        {/if}
      {:else}
        <p class="empty">Run a query to see results.</p>
      {/if}
    </div>
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.4);
    backdrop-filter: blur(2px);
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 8vh;
    z-index: 1000;
  }

  .console-modal {
    width: 100%;
    max-width: 960px;
    max-height: 84vh;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-lg, 12px);
    box-shadow: 0 10px 40px rgba(0, 0, 0, 0.2);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 1rem 1.25rem;
    border-bottom: 1px solid var(--border);
  }

  .modal-header h2 {
    margin: 0;
    font-size: 1.1rem;
    font-weight: 600;
  }

  .close-btn {
    background: none;
    border: none;
    cursor: pointer;
    color: var(--muted-foreground);
    padding: 0.25rem;
    border-radius: var(--radius);
  }

  .close-btn:hover {
    background: var(--accent-hover);
    color: var(--fg);
  }

  .query-wrap {
    padding: var(--p-3) var(--p-4) 0;
  }

  .query-input,
  .turtle,
  .error,
  .iri,
  .bnode,
  .literal-hint {
    font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
  }

  .query-input {
    width: 100%;
    resize: vertical;
    min-height: 6rem;
    padding: var(--p-2) var(--p-3);
    font-size: 0.85rem;
    line-height: 1.5;
    color: var(--fg);
    background: var(--editor-bg);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    outline: none;
  }

  .query-input:focus {
    border-color: var(--primary);
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: var(--p-3);
    padding: var(--p-3) var(--p-4);
    border-bottom: 1px solid var(--border);
    font-size: 0.8rem;
    color: var(--muted-foreground);
  }

  .run-btn {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.4rem 0.9rem;
    background: var(--primary);
    color: var(--primary-foreground);
    border: none;
    border-radius: var(--radius);
    cursor: pointer;
    font-size: 0.8125rem;
    font-weight: 600;
    transition: background 0.15s;
  }

  .run-btn:hover:not(:disabled) {
    background: var(--primary-hover);
  }

  .run-btn:disabled {
    opacity: 0.6;
    cursor: default;
  }

  .hint {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .status {
    flex-shrink: 0;
    font-variant-numeric: tabular-nums;
  }

  .results {
    flex: 1;
    min-height: 8rem;
    overflow: auto;
    padding: var(--p-3) var(--p-4);
  }

  .empty,
  .notice {
    margin: 0 0 var(--p-2);
    font-size: 0.85rem;
    color: var(--muted-foreground);
  }

  .notice {
    padding: var(--p-2) var(--p-3);
    background: var(--accent);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--fg);
  }

  /* Same literal red as broken wikilinks; there is no warning token yet. */
  .error {
    margin: 0;
    padding: var(--p-2) var(--p-3);
    white-space: pre-wrap;
    font-size: 0.8rem;
    color: #dc2626;
    background: color-mix(in srgb, #dc2626 8%, transparent);
    border: 1px solid color-mix(in srgb, #dc2626 35%, transparent);
    border-radius: var(--radius);
  }

  .boolean {
    margin: 0;
    font-size: 1.25rem;
    font-weight: 600;
  }

  .turtle {
    margin: 0;
    white-space: pre;
    font-size: 0.8rem;
    line-height: 1.5;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.85rem;
  }

  th {
    position: sticky;
    top: 0;
    background: var(--bg);
    text-align: left;
    font-weight: 600;
    color: var(--muted-foreground);
    padding: var(--p-1) var(--p-2);
    border-bottom: 1px solid var(--border);
  }

  td {
    padding: var(--p-1) var(--p-2);
    border-bottom: 1px solid var(--border);
    vertical-align: top;
    overflow-wrap: anywhere;
  }

  tbody tr:hover {
    background: var(--accent-hover);
  }

  .page-link {
    padding: 0;
    background: none;
    border: none;
    cursor: pointer;
    font: inherit;
    text-align: left;
    color: var(--primary);
  }

  .page-link:hover {
    text-decoration: underline;
  }

  .iri,
  .bnode {
    font-size: 0.8rem;
  }

  .literal-hint {
    margin-left: 0.25rem;
    font-size: 0.75rem;
    color: var(--muted-foreground);
  }

  kbd {
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 3px;
    padding: 0 4px;
    font-family: inherit;
    font-size: 0.8em;
    box-shadow: 0 1px 0 rgba(0, 0, 0, 0.1);
  }
</style>
