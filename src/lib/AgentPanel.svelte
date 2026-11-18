<script lang="ts">
  /**
   * The Agent sub-panel. A placeholder in `add-right-panel-tabs`: it shows the
   * connection status and nothing else; `add-agent-panel` fills it in.
   */
  import { app } from './stores.svelte';
  import { AGENT_STATUS_LABELS } from './right-panels';

  let { visible = true }: { visible?: boolean } = $props();
</script>

<div class="agent-panel" data-testid="agent-panel" data-visible={visible}>
  <div class="panel-content">
    <h3 class="panel-title">Agent</h3>
    <p class="status" data-status={app.agentStatus} role="status">
      <span class="dot" aria-hidden="true"></span>
      {AGENT_STATUS_LABELS[app.agentStatus]}
    </p>
    <p class="empty">The agent chat is not available yet.</p>
  </div>
</div>

<style>
  .agent-panel {
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

  .status {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin: 0 0 0.5rem;
    font-size: 0.8125rem;
    color: var(--fg);
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    border: 1.5px solid var(--muted-foreground);
    background: transparent;
    flex-shrink: 0;
  }

  .status[data-status='connected'] .dot {
    background: var(--primary);
    border-color: var(--primary);
  }

  .empty {
    margin: 0;
    color: var(--muted-foreground);
    font-size: 0.8125rem;
    font-style: italic;
  }
</style>
