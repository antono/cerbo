<script lang="ts">
  /**
   * An approval card: what the agent wants to do, the difference, and
   * Approve / Reject. After an answer the card shows the outcome.
   */
  import { diffLines, type FeedItem } from './feed';
  import { ACTION_LABELS as OTHER_ACTION_LABELS } from './host';
  import type { ApprovalOutcome } from './types';

  let {
    item,
    onAnswer,
    live = true,
  }: {
    item: Extract<FeedItem, { kind: 'approval' }>;
    onAnswer: (approval: number, approve: boolean) => void | Promise<void>;
    /** The daemon still lists this approval as waiting; false hides the buttons. */
    live?: boolean;
  } = $props();

  let approval = $derived(item.approval);
  let isTool = $derived(approval.action === 'tool');
  let lines = $derived(approval.diff ? diffLines(approval.diff) : []);
  let answering = $state(false);

  const OUTCOME_LABEL: Record<ApprovalOutcome, string> = {
    approved: 'Applied',
    rejected: 'Rejected',
    timed_out: 'Timed out',
    cancelled: 'Cancelled',
  };

  const ACTION_LABEL: Record<string, string> = {
    create: 'Create page',
    edit: 'Edit page',
    rename: 'Rename page',
    move: 'Move page',
    trash: 'Move page to trash',
    link: 'Add link',
    tool: 'Use tool',
  };

  let heading = $derived(
    isTool ? `${ACTION_LABEL.tool}: ${approval.tool_title ?? approval.summary}` : (ACTION_LABEL[approval.action] ?? OTHER_ACTION_LABELS[approval.action] ?? approval.action),
  );

  async function answer(approve: boolean) {
    answering = true;
    try {
      await onAnswer(approval.id, approve);
    } finally {
      answering = false;
    }
  }
</script>

<div
  class="approval-card"
  data-testid="agent-approval-card"
  data-approval-id={approval.id}
  data-outcome={item.outcome ?? 'pending'}
  role="group"
  aria-label={heading}
>
  <div class="head">
    <span class="title">{heading}</span>
    {#if item.outcome}
      <span class="outcome" data-testid="agent-approval-outcome" data-outcome={item.outcome}>
        {OUTCOME_LABEL[item.outcome]}
      </span>
    {/if}
  </div>
  {#if !isTool}<p class="summary">{approval.summary}</p>{/if}
  {#if isTool && approval.tool_kind}<p class="summary">{approval.tool_kind}</p>{/if}
  {#if lines.length > 0}
    <!-- svelte-ignore a11y_no_noninteractive_tabindex (a scrollable region must take focus) -->
    <pre class="diff" data-testid="agent-approval-diff" tabindex="0" aria-label="Changes">{#each lines as line, i (i)}<span class="line {line.kind}">{line.text || ' '}</span>{/each}</pre>
  {/if}
  {#if !item.outcome && !live}
    <p class="summary" data-testid="agent-approval-stale">No longer waiting for an answer</p>
  {/if}
  {#if !item.outcome && live}
    <div class="actions">
      <button class="btn approve" data-testid="agent-approve" disabled={answering} onclick={() => answer(true)}>
        Approve
      </button>
      <button class="btn reject" data-testid="agent-reject" disabled={answering} onclick={() => answer(false)}>
        Reject
      </button>
    </div>
  {/if}
</div>

<style>
  .approval-card {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    padding: 0.6rem;
    border: 1px solid var(--border);
    border-left: 3px solid #f59e0b;
    border-radius: var(--radius);
    background: var(--bg);
    font-size: 0.8125rem;
  }

  .approval-card[data-outcome='approved'] {
    border-left-color: #16a34a;
  }

  .approval-card[data-outcome='rejected'],
  .approval-card[data-outcome='cancelled'] {
    border-left-color: var(--muted-foreground);
  }

  .head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 0.5rem;
  }

  .title {
    font-weight: 600;
    min-width: 0;
    overflow-wrap: anywhere;
  }

  .outcome {
    flex-shrink: 0;
    font-size: 0.7rem;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    color: var(--muted-foreground);
  }

  .outcome[data-outcome='approved'] {
    color: #16a34a;
  }

  .summary {
    margin: 0;
    color: var(--muted-foreground);
    overflow-wrap: anywhere;
  }

  .diff {
    margin: 0;
    max-height: 14rem;
    overflow: auto;
    padding: 0.4rem;
    border-radius: var(--radius);
    background: var(--accent);
    font-size: 0.72rem;
    line-height: 1.4;
  }

  .line {
    display: block;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .line.add {
    background: rgb(22 163 74 / 0.18);
  }

  .line.remove {
    background: rgb(239 68 68 / 0.18);
  }

  .line.hunk {
    color: var(--muted-foreground);
  }

  .actions {
    display: flex;
    gap: 0.4rem;
  }

  .btn {
    padding: 0.3rem 0.8rem;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--accent);
    color: var(--fg);
    font-size: 0.8rem;
    cursor: pointer;
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: default;
  }

  .btn.approve {
    background: var(--primary);
    border-color: var(--primary);
    color: var(--primary-foreground);
  }

  .btn:not(:disabled):hover {
    filter: brightness(0.95);
  }

  .btn:focus-visible {
    outline: 2px solid var(--primary);
    outline-offset: 1px;
  }
</style>
