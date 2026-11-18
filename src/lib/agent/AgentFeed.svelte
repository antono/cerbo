<script lang="ts">
  /**
   * The chat feed: built from the session's events, kept at the newest
   * message unless the person scrolled up, and capped so a long session does
   * not render thousands of rows.
   */
  import { tick } from 'svelte';
  import { ArrowDown } from 'lucide-svelte';
  import { app, openPage } from '../stores.svelte';
  import { FEED_WINDOW, windowItems, type FeedItem } from './feed';
  import ApprovalCard from './ApprovalCard.svelte';
  import ToolRow from './ToolRow.svelte';

  let {
    items,
    loading = false,
    onAnswer,
  }: {
    items: FeedItem[];
    loading?: boolean;
    onAnswer: (approval: number, approve: boolean) => void | Promise<void>;
  } = $props();

  /** How many items the cap allows; "Show earlier" raises it. */
  let limit = $state(FEED_WINDOW);
  let windowed = $derived(windowItems(items, limit));

  let scroller = $state<HTMLDivElement | null>(null);
  /** At the bottom (within a few pixels): new content keeps the view there. */
  let stuck = $state(true);
  const NEAR_BOTTOM = 24;

  function onScroll() {
    if (!scroller) return;
    stuck = scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight <= NEAR_BOTTOM;
  }

  async function toBottom() {
    stuck = true;
    await tick();
    if (scroller) scroller.scrollTop = scroller.scrollHeight;
  }

  // Follow the newest content, and only while at the bottom.
  $effect(() => {
    // Streaming grows the last item's text, not the list: read both.
    void items.length;
    void items[items.length - 1];
    if (stuck) void toBottom();
  });

  async function showEarlier() {
    // Keep what the person reads in place while older items appear above.
    const before = scroller ? scroller.scrollHeight - scroller.scrollTop : 0;
    limit += FEED_WINDOW;
    await tick();
    if (scroller) scroller.scrollTop = scroller.scrollHeight - before;
  }

  function pageTitle(path: string): string {
    return app.pages.find((p) => p.uuid === path)?.title ?? path;
  }
</script>

<div class="feed-wrap">
  <div
    class="feed"
    data-testid="agent-feed"
    data-stuck={stuck}
    role="log"
    aria-live="polite"
    aria-label="Agent conversation"
    bind:this={scroller}
    onscroll={onScroll}
  >
    {#if windowed.hidden > 0}
      <button class="earlier" data-testid="agent-show-earlier" onclick={showEarlier}>
        Show {Math.min(windowed.hidden, FEED_WINDOW)} earlier of {windowed.hidden}
      </button>
    {/if}

    {#each windowed.shown as item (item.key)}
      {#if item.kind === 'user'}
        <div class="message user" data-testid="agent-message" data-role="user">
          <p class="text">{item.text}</p>
          {#if item.context.length > 0}
            <ul class="context" aria-label="Context pages">
              {#each item.context as ref (ref.path)}
                <li>{ref.title}</li>
              {/each}
            </ul>
          {/if}
        </div>
      {:else if item.kind === 'agent'}
        <div class="message agent" data-testid="agent-message" data-role="agent">
          <p class="text">{item.text}</p>
        </div>
      {:else if item.kind === 'thought'}
        <details class="thought" data-testid="agent-thought">
          <summary>Thinking</summary>
          <p class="text">{item.text}</p>
        </details>
      {:else if item.kind === 'tool'}
        <ToolRow {item} />
      {:else if item.kind === 'approval'}
        <ApprovalCard {item} {onAnswer} />
      {:else if item.kind === 'page'}
        <button class="page-link" data-testid="agent-page-link" data-page={item.path} onclick={() => openPage(item.path)}>
          Changed: <span class="page-title">{pageTitle(item.path)}</span>
        </button>
      {:else if item.kind === 'notice'}
        <div class="notice" data-level={item.level} data-testid="agent-notice" role={item.level === 'error' ? 'alert' : 'status'}>
          <span>{item.text}</span>
          {#if item.detail}
            <details>
              <summary>Agent output</summary>
              <pre>{item.detail}</pre>
            </details>
          {/if}
        </div>
      {/if}
    {/each}

    {#if items.length === 0}
      <p class="hint" data-testid="agent-feed-empty">
        {loading ? 'Loading the session…' : 'Ask the agent about this vault.'}
      </p>
    {/if}
  </div>

  {#if !stuck}
    <button class="jump" data-testid="agent-jump-latest" aria-label="Jump to the newest message" onclick={toBottom}>
      <ArrowDown size={14} />
    </button>
  {/if}
</div>

<style>
  .feed-wrap {
    position: relative;
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }

  .feed {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 0.75rem;
  }

  .message {
    max-width: 100%;
    font-size: 0.8125rem;
    line-height: 1.45;
  }

  .message.user {
    align-self: flex-end;
    max-width: 92%;
    padding: 0.4rem 0.6rem;
    border-radius: var(--radius);
    background: var(--primary);
    color: var(--primary-foreground);
  }

  .text {
    margin: 0;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .context {
    list-style: none;
    margin: 0.3rem 0 0;
    padding: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem;
    font-size: 0.7rem;
    opacity: 0.85;
  }

  .context li {
    padding: 0 0.4rem;
    border: 1px solid currentColor;
    border-radius: 999px;
  }

  .thought {
    font-size: 0.75rem;
    color: var(--muted-foreground);
  }

  .thought summary {
    cursor: pointer;
  }

  .thought .text {
    margin-top: 0.25rem;
    font-style: italic;
  }

  .page-link {
    align-self: flex-start;
    max-width: 100%;
    padding: 0.2rem 0.5rem;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg);
    color: var(--fg);
    font-size: 0.75rem;
    cursor: pointer;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .page-title {
    color: var(--primary);
    font-weight: 600;
  }

  .page-link:hover,
  .page-link:focus-visible {
    background: var(--accent-hover);
    outline: none;
  }

  .notice {
    font-size: 0.75rem;
    color: var(--muted-foreground);
  }

  .notice[data-level='error'] {
    color: #ef4444;
  }

  .notice pre {
    margin: 0.25rem 0 0;
    max-height: 10rem;
    overflow: auto;
    padding: 0.4rem;
    border-radius: var(--radius);
    background: var(--accent);
    color: var(--fg);
    font-size: 0.7rem;
    white-space: pre-wrap;
  }

  .earlier {
    align-self: center;
    padding: 0.2rem 0.6rem;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--bg);
    color: var(--muted-foreground);
    font-size: 0.72rem;
    cursor: pointer;
  }

  .hint {
    margin: auto 0;
    text-align: center;
    color: var(--muted-foreground);
    font-size: 0.8125rem;
    font-style: italic;
  }

  .jump {
    position: absolute;
    right: 0.75rem;
    bottom: 0.5rem;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border: 1px solid var(--border);
    border-radius: 50%;
    background: var(--bg);
    color: var(--fg);
    box-shadow: 0 1px 4px rgb(0 0 0 / 0.2);
    cursor: pointer;
  }
</style>
