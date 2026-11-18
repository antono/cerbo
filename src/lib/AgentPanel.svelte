<script lang="ts">
  /**
   * The Agent sub-panel (change `add-agent-panel`, spec `desktop-agent-panel`):
   * a chat with an agent working in the open vault. The state lives in
   * `agentStore` (started by the layout, so the tab mark works while this panel
   * is hidden); this component only arranges the parts.
   */
  import { untrack } from 'svelte';
  import { agentStore as store } from './agent/store.svelte';
  import AgentComposer from './agent/AgentComposer.svelte';
  import AgentFeed from './agent/AgentFeed.svelte';
  import AgentHeader from './agent/AgentHeader.svelte';
  import EmptyState from './agent/EmptyState.svelte';
  import PermissionControl from './agent/PermissionControl.svelte';
  import SessionsList from './agent/SessionsList.svelte';

  let { visible = true }: { visible?: boolean } = $props();

  let sessionsOpen = $state(false);

  // Look again whenever the tab is opened: an agent may have been installed.
  $effect(() => {
    if (visible && store.connection.state === 'connected') untrack(() => void store.refreshAgents());
  });

  let info = $derived(store.activeInfo);
  let warning = $derived(info && (info.sandboxed === false || info.warning) ? (info.warning ?? 'The agent runs without a sandbox.') : null);

  async function openSession(id: string) {
    sessionsOpen = false;
    await store.openSession(id);
  }
</script>

<div class="agent-panel" data-testid="agent-panel" data-visible={visible} data-view={store.panelView}>
  <AgentHeader
    agents={store.agents}
    selected={store.agent}
    connection={store.connection.state}
    {sessionsOpen}
    onPick={(kind) => store.chooseAgent(kind)}
    onNew={() => store.newSession()}
    onToggleSessions={() => (sessionsOpen = !sessionsOpen)}
  />

  {#if sessionsOpen}
    <SessionsList
      sessions={store.sessions}
      activeId={store.activeId}
      onOpen={openSession}
      onDelete={(id) => store.deleteSession(id)}
    />
  {/if}

  {#if store.panelView === 'ready'}
    <PermissionControl value={store.permission} onChange={(level) => store.setPermission(level)} />
    {#if warning}
      <p class="warning" data-testid="agent-sandbox-warning" role="alert">{warning}</p>
    {/if}
    <AgentFeed items={store.view.items} loading={store.loadingEvents || store.busy} onAnswer={(id, ok) => store.answerApproval(id, ok)} />
    {#if store.error}
      <p class="error" data-testid="agent-error" role="alert">{store.error}</p>
    {/if}
    <AgentComposer
      working={store.working}
      starting={store.starting}
      onSend={(text, context) => store.send(text, context)}
      onCancel={() => store.cancel()}
    />
  {:else}
    <EmptyState
      view={store.panelView}
      message={store.connection.message}
      agents={store.agents ?? []}
      selected={store.agent}
      onRetry={() => store.retry()}
      onRecheck={() => store.refreshAgents()}
    />
    {#if store.error && store.panelView !== 'daemon_unavailable'}
      <p class="error" data-testid="agent-error" role="alert">{store.error}</p>
    {/if}
  {/if}
</div>

<style>
  .agent-panel {
    height: 100%;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .warning,
  .error {
    margin: 0;
    padding: 0.4rem 0.75rem;
    font-size: 0.75rem;
    flex-shrink: 0;
  }

  .warning {
    background: rgb(245 158 11 / 0.15);
    border-bottom: 1px solid #f59e0b;
    color: var(--fg);
  }

  .error {
    color: #ef4444;
    overflow-wrap: anywhere;
  }
</style>
