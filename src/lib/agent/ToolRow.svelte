<script lang="ts">
  /** One compact row per tool call: name, short arguments, result state. */
  import { Check, LoaderCircle, X } from 'lucide-svelte';
  import { toolStatusKind, type FeedItem } from './feed';

  let { item }: { item: Extract<FeedItem, { kind: 'tool' }> } = $props();

  let state = $derived(toolStatusKind(item.status));
  const STATE_LABEL = { running: 'Running', done: 'Done', failed: 'Failed' } as const;
</script>

<div
  class="tool-row"
  data-testid="agent-tool-row"
  data-status={state}
  data-tool-kind={item.toolKind}
  title={item.output ?? item.input ?? item.title}
>
  <span class="icon" aria-label={STATE_LABEL[state]} role="img">
    {#if state === 'running'}
      <LoaderCircle size={12} class="spin" />
    {:else if state === 'done'}
      <Check size={12} />
    {:else}
      <X size={12} />
    {/if}
  </span>
  <span class="name">{item.title}</span>
  {#if item.input}<span class="args">{item.input}</span>{/if}
  {#if item.output}<span class="result">{item.output}</span>{/if}
</div>

<style>
  .tool-row {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    min-width: 0;
    padding: 0.2rem 0.5rem;
    border-radius: var(--radius);
    background: var(--accent);
    color: var(--muted-foreground);
    font-size: 0.75rem;
  }

  .icon {
    display: flex;
    flex-shrink: 0;
  }

  .tool-row[data-status='done'] .icon {
    color: #16a34a;
  }

  .tool-row[data-status='failed'] .icon {
    color: #ef4444;
  }

  .name {
    font-weight: 600;
    color: var(--fg);
    flex-shrink: 0;
  }

  .args,
  .result {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-width: 0;
  }

  .args {
    flex: 1 1 auto;
  }

  .result {
    flex: 0 1 auto;
    font-style: italic;
  }

  :global(.spin) {
    animation: agent-spin 1s linear infinite;
  }

  @keyframes agent-spin {
    to {
      transform: rotate(360deg);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    :global(.spin) {
      animation: none;
    }
  }
</style>
