<script lang="ts">
  /** The tab header: agent picker, connection state, New session and the Sessions list. */
  import { History, Plus } from 'lucide-svelte';
  import { AGENT_LABELS, type ConnectionState } from './status';
  import type { AgentInfo, AgentKind } from './types';

  let {
    agents,
    selected,
    connection,
    sessionsOpen,
    onPick,
    onNew,
    onToggleSessions,
  }: {
    agents: AgentInfo[] | null;
    selected: AgentKind | null;
    connection: ConnectionState;
    sessionsOpen: boolean;
    onPick: (agent: AgentKind) => void;
    onNew: () => void;
    onToggleSessions: () => void;
  } = $props();

  const KINDS: AgentKind[] = ['opencode', 'claude'];
  const CONNECTION_LABEL: Record<ConnectionState, string> = {
    connected: 'Connected',
    connecting: 'Connecting…',
    unavailable: 'Not connected',
  };

  function available(kind: AgentKind): boolean {
    return agents?.find((a) => a.agent === kind)?.available ?? false;
  }
</script>

<div class="header" data-testid="agent-header">
  <select
    class="picker"
    data-testid="agent-picker"
    aria-label="Agent"
    disabled={!agents}
    value={selected ?? ''}
    onchange={(e) => onPick(e.currentTarget.value as AgentKind)}
  >
    {#if selected === null}<option value="" disabled>No agent</option>{/if}
    {#each KINDS as kind (kind)}
      <option value={kind} disabled={!available(kind)}>
        {AGENT_LABELS[kind]}{available(kind) ? '' : ' (not found)'}
      </option>
    {/each}
  </select>

  <span class="conn" data-testid="agent-connection" data-state={connection} role="status">
    <span class="dot" aria-hidden="true"></span>
    <span class="conn-label">{CONNECTION_LABEL[connection]}</span>
  </span>

  <button class="icon-btn" data-testid="agent-new-session" aria-label="New session" title="New session" onclick={onNew}>
    <Plus size={15} />
  </button>
  <button
    class="icon-btn"
    class:on={sessionsOpen}
    data-testid="agent-sessions-toggle"
    aria-label="Sessions"
    aria-expanded={sessionsOpen}
    title="Sessions"
    onclick={onToggleSessions}
  >
    <History size={15} />
  </button>
</div>

<style>
  .header {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.4rem 0.75rem;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .picker {
    min-width: 0;
    flex: 1 1 auto;
    padding: 0.2rem 0.3rem;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg);
    color: var(--fg);
    font-size: 0.75rem;
  }

  .conn {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    flex-shrink: 0;
    font-size: 0.7rem;
    color: var(--muted-foreground);
  }

  .dot {
    width: 7px;
    height: 7px;
    box-sizing: border-box;
    border-radius: 50%;
    border: 1.5px solid var(--muted-foreground);
  }

  .conn[data-state='connected'] .dot {
    background: var(--primary);
    border-color: var(--primary);
  }

  .icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    flex-shrink: 0;
    padding: 0;
    border: none;
    border-radius: var(--radius);
    background: none;
    color: var(--muted-foreground);
    cursor: pointer;
  }

  .icon-btn:hover,
  .icon-btn:focus-visible,
  .icon-btn.on {
    background: var(--accent-hover);
    color: var(--fg);
    outline: none;
  }

  @media (max-width: 320px) {
    .conn-label {
      display: none;
    }
  }
</style>
