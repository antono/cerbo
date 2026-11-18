<script lang="ts">
  /**
   * The vault's agent permission as a three-way switch. Widening it (to let
   * the agent do more) asks first.
   */
  import ConfirmationDialog from '../ConfirmationDialog.svelte';
  import {
    isWidening,
    PERMISSIONS,
    PERMISSION_HINTS,
    PERMISSION_LABELS,
    wideningWarning,
    type AgentPermission,
  } from './permission';

  let {
    value,
    onChange,
  }: {
    value: AgentPermission;
    onChange: (level: AgentPermission) => void | Promise<void>;
  } = $props();

  let asking = $state<AgentPermission | null>(null);

  function choose(level: AgentPermission) {
    if (level === value) return;
    if (isWidening(value, level)) asking = level;
    else void onChange(level);
  }

  async function confirm() {
    const level = asking;
    asking = null;
    if (level) await onChange(level);
  }
</script>

<div class="permission" data-testid="agent-permission" data-value={value}>
  <span class="label" id="agent-permission-label">Agent may</span>
  <div class="segments" role="radiogroup" aria-labelledby="agent-permission-label">
    {#each PERMISSIONS as level (level)}
      <button
        class="segment"
        role="radio"
        aria-checked={value === level}
        title={PERMISSION_HINTS[level]}
        data-testid="agent-permission-{level}"
        onclick={() => choose(level)}
      >
        {PERMISSION_LABELS[level]}
      </button>
    {/each}
  </div>
</div>

{#if asking}
  <div data-testid="agent-permission-confirm">
    <ConfirmationDialog
      title="Let the agent do more?"
      message={`${PERMISSION_LABELS[value]} to ${PERMISSION_LABELS[asking]}. ${wideningWarning(asking)}`}
      confirmLabel="Allow"
      onConfirm={confirm}
      onClose={() => (asking = null)}
    />
  </div>
{/if}

<style>
  .permission {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.4rem 0.75rem;
    border-bottom: 1px solid var(--border);
    font-size: 0.75rem;
  }

  .label {
    color: var(--muted-foreground);
    flex-shrink: 0;
  }

  .segments {
    display: flex;
    flex: 1;
    min-width: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }

  .segment {
    flex: 1;
    padding: 0.2rem 0.3rem;
    border: none;
    border-right: 1px solid var(--border);
    background: var(--bg);
    color: var(--muted-foreground);
    font-size: 0.72rem;
    cursor: pointer;
    white-space: nowrap;
  }

  .segment:last-child {
    border-right: none;
  }

  .segment[aria-checked='true'] {
    background: var(--primary);
    color: var(--primary-foreground);
  }

  .segment:not([aria-checked='true']):hover,
  .segment:focus-visible {
    background: var(--accent-hover);
    color: var(--fg);
    outline: none;
  }
</style>
