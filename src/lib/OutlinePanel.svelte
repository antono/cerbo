<script lang="ts">
  /**
   * The Outline sub-panel: the open page's headings, indented by level, read
   * from the live editor text so the list follows typing (spec
   * `desktop-outline-panel`). Clicking a heading jumps the editor there; the
   * heading at the top of the editor is marked. Stays mounted while another
   * tab is selected and does no work then (`visible`, design D3).
   */
  import { app } from './stores.svelte';
  import { headingsFor, outlineDepths, type Heading } from './outline';
  import { jumpToHeading } from './outline-bridge';

  let { uuid, visible = true }: { uuid: string; visible?: boolean } = $props();

  // The last list drawn; kept while hidden so a hidden panel parses nothing.
  let shown: Heading[] = [];
  let headings = $derived.by(() => {
    if (!visible) return shown;
    shown = app.pageTextUuid === uuid ? headingsFor(app.pageText) : [];
    return shown;
  });
  let depths = $derived(outlineDepths(headings));
</script>

<div class="outline-panel" data-testid="outline-panel">
  <div class="panel-content">
    <h3 class="panel-title">Outline</h3>
    {#if headings.length === 0}
      <p class="empty" data-testid="outline-empty">This page has no headings.</p>
    {:else}
      <ul class="outline-list">
        {#each headings as heading, i (heading.offset)}
          <li>
            <button
              class="outline-item"
              data-testid="outline-item"
              data-level={heading.level}
              aria-current={app.outlineCurrent === i ? 'true' : undefined}
              style:padding-left="{0.6 + depths[i] * 0.9}rem"
              title={heading.text}
              onclick={() => jumpToHeading(i)}
            >
              {heading.text}
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</div>

<style>
  .outline-panel {
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .panel-content {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 1rem;
  }

  .panel-title {
    margin: 0 0 0.5rem;
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

  .outline-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.125rem;
  }

  .outline-item {
    background: none;
    border: none;
    border-left: 2px solid transparent;
    padding: 0.35rem 0.6rem;
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

  .outline-item:hover,
  .outline-item:focus-visible {
    background: var(--accent-hover);
    color: var(--primary);
    outline: none;
  }

  .outline-item[aria-current='true'] {
    border-left-color: var(--primary);
    color: var(--primary);
    font-weight: 600;
  }
</style>
