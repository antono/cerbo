<script lang="ts">
  /** The sessions of the open vault, newest first: open one, or delete an ended one. */
  import { Trash2 } from 'lucide-svelte';
  import { ageLabel, isLive } from './events';
  import { AGENT_LABELS } from './status';
  import type { SessionInfo } from './types';

  let {
    sessions,
    activeId,
    onOpen,
    onDelete,
  }: {
    sessions: SessionInfo[];
    activeId: string | null;
    onOpen: (id: string) => void;
    onDelete: (id: string) => void;
  } = $props();

  const now = Date.now();
</script>

<div class="sessions" data-testid="agent-sessions" role="menu" aria-label="Sessions">
  {#if sessions.length === 0}
    <p class="empty">No sessions yet.</p>
  {:else}
    <ul>
      {#each sessions as s (s.id)}
        <li class="item" class:active={s.id === activeId}>
          <button
            class="open"
            role="menuitem"
            data-testid="agent-session-item"
            data-session={s.id}
            data-state={s.state}
            onclick={() => onOpen(s.id)}
          >
            <span class="title">{s.title || 'New session'}</span>
            <span class="meta">{AGENT_LABELS[s.agent]} · {s.state} · {ageLabel(s.updated_at, now)}</span>
          </button>
          {#if !isLive(s.state)}
            <button
              class="delete"
              data-testid="agent-session-delete"
              aria-label="Delete session {s.title || 'New session'}"
              title="Delete"
              onclick={() => onDelete(s.id)}
            >
              <Trash2 size={13} />
            </button>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .sessions {
    max-height: 16rem;
    overflow-y: auto;
    border-bottom: 1px solid var(--border);
    background: var(--bg);
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0.25rem;
  }

  .item {
    display: flex;
    align-items: center;
    border-radius: var(--radius);
  }

  .item.active {
    background: var(--accent);
  }

  .open {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    padding: 0.3rem 0.5rem;
    border: none;
    border-radius: var(--radius);
    background: none;
    color: var(--fg);
    text-align: left;
    cursor: pointer;
  }

  .open:hover,
  .open:focus-visible {
    background: var(--accent-hover);
    outline: none;
  }

  .title {
    font-size: 0.8125rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .meta {
    font-size: 0.7rem;
    color: var(--muted-foreground);
  }

  .delete {
    display: flex;
    padding: 0.35rem;
    border: none;
    border-radius: var(--radius);
    background: none;
    color: var(--muted-foreground);
    cursor: pointer;
  }

  .delete:hover,
  .delete:focus-visible {
    background: var(--accent-hover);
    color: #ef4444;
    outline: none;
  }

  .empty {
    margin: 0;
    padding: 0.6rem 0.75rem;
    font-size: 0.8125rem;
    font-style: italic;
    color: var(--muted-foreground);
  }
</style>
