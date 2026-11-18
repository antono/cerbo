<script lang="ts">
  /** The Agent tab when there is no chat to show: daemon, agent or login missing. */
  import { AGENT_COMMANDS, AGENT_LABELS, AGENT_LOGIN_HINTS, type PanelView } from './status';
  import type { AgentInfo, AgentKind } from './types';

  let {
    view,
    message = '',
    agents = [],
    selected = null,
    onRetry,
    onRecheck,
  }: {
    view: Exclude<PanelView, 'ready'>;
    /** Why the daemon is unavailable. */
    message?: string;
    agents?: AgentInfo[];
    selected?: AgentKind | null;
    onRetry: () => void | Promise<void>;
    onRecheck: () => void | Promise<void>;
  } = $props();

  const KINDS: AgentKind[] = ['opencode', 'claude'];
  let signedOut = $derived(agents.find((a) => a.available && a.agent === (selected ?? a.agent) && a.logged_in === false));
</script>

{#if view === 'connecting'}
  <div class="empty" data-testid="agent-empty-connecting" role="status" aria-busy="true">
    <p>Connecting to the Cerbo daemon…</p>
  </div>
{:else if view === 'daemon_unavailable'}
  <div class="empty" data-testid="agent-empty-daemon" role="alert">
    <p class="title">The Cerbo daemon is not reachable</p>
    <p>The agent runs in the background daemon, <code>cerboid</code>. Cerbo tried to start it and could not connect.</p>
    {#if message}<p class="reason" data-testid="agent-empty-reason">{message}</p>{/if}
    <button class="btn" data-testid="agent-retry" onclick={() => onRetry()}>Retry</button>
  </div>
{:else if view === 'no_agent'}
  <div class="empty" data-testid="agent-empty-no-agent">
    <p class="title">No agent installed</p>
    <p>Install one of these, then check again. Cerbo looks for the command on your <code>PATH</code>.</p>
    <ul class="agents">
      {#each KINDS as kind (kind)}
        {@const info = agents.find((a) => a.agent === kind)}
        <li data-agent={kind}>
          <span class="name">{AGENT_LABELS[kind]}</span>
          <code>{AGENT_COMMANDS[kind]}</code>
          {#if info?.reason}<span class="reason">{info.reason}</span>{/if}
        </li>
      {/each}
    </ul>
    <button class="btn" data-testid="agent-recheck" onclick={() => onRecheck()}>Check again</button>
  </div>
{:else if view === 'not_logged_in'}
  <div class="empty" data-testid="agent-empty-login">
    <p class="title">{signedOut ? AGENT_LABELS[signedOut.agent] : 'The agent'} is not signed in</p>
    {#if signedOut}
      <p>Sign in from a terminal: <code>{AGENT_LOGIN_HINTS[signedOut.agent]}</code>. Then check again.</p>
    {/if}
    <button class="btn" data-testid="agent-recheck" onclick={() => onRecheck()}>Check again</button>
  </div>
{/if}

<style>
  .empty {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.5rem;
    padding: 1rem;
    font-size: 0.8125rem;
    color: var(--fg);
  }

  p {
    margin: 0;
  }

  .title {
    font-weight: 700;
  }

  .reason {
    color: var(--muted-foreground);
    font-size: 0.75rem;
    overflow-wrap: anywhere;
  }

  code {
    padding: 0 0.3rem;
    border-radius: 3px;
    background: var(--accent);
    font-size: 0.75rem;
  }

  .agents {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    width: 100%;
  }

  .agents li {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 0.4rem;
  }

  .name {
    font-weight: 600;
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

  .btn:hover,
  .btn:focus-visible {
    background: var(--accent-hover);
    outline: none;
  }
</style>
