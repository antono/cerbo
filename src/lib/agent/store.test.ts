import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { EDIT_APPROVAL, log } from './fixtures';
import type { AgentInfo, SessionEvent, SessionInfo } from './types';

// The Tauri bridge, scripted: `commands` answers `invoke`, `emit` plays what
// the desktop would forward from the daemon.
const commands: Record<string, (args: Record<string, unknown>) => unknown> = {};
const calls: { command: string; args: Record<string, unknown> }[] = [];
const listeners = new Map<string, (e: { payload: unknown }) => void>();

vi.mock('@tauri-apps/api/core', () => ({
  invoke: async (command: string, args: Record<string, unknown> = {}) => {
    calls.push({ command, args });
    const handler = commands[command];
    if (!handler) throw { code: 'failed', message: `unscripted command ${command}` };
    const result = handler(args);
    if (result instanceof Error) throw { code: 'failed', message: result.message };
    return result;
  },
}));
vi.mock('@tauri-apps/api/event', () => ({
  listen: async (name: string, cb: (e: { payload: unknown }) => void) => {
    listeners.set(name, cb);
    return () => listeners.delete(name);
  },
}));
vi.mock('../stores.svelte', () => ({
  app: {
    activeVaultId: 'v1',
    agentStatus: 'disconnected',
    pages: [{ uuid: 'p-vendor', title: 'Vendor onboarding', virtualPath: null }],
    currentUuid: null as string | null,
    currentContent: '',
    pageUnsaved: false,
  },
  loadPages: vi.fn(async () => {}),
  loadVaultObjects: vi.fn(async () => {}),
}));

const { app } = await import('../stores.svelte');
const { AgentStore } = await import('./store.svelte');

const AGENTS: AgentInfo[] = [
  { agent: 'opencode', available: true, logged_in: true },
  { agent: 'claude', available: false, reason: 'not found' },
];
const session = (id: string, state: SessionInfo['state'] = 'idle'): SessionInfo => ({
  id,
  vault: 'v1',
  agent: 'opencode',
  title: id,
  state,
  created_at: '2026-11-18T10:00:00Z',
  updated_at: '2026-11-18T10:00:00Z',
  resumable: true,
  sandboxed: true,
});

function emit(name: string, payload: unknown) {
  listeners.get(name)!({ payload });
}
function live(sessionId: string, events: SessionEvent[]) {
  for (const event of events) emit('agent-event', { kind: 'session', vault: 'v1', session: sessionId, event });
}
async function settle() {
  await vi.advanceTimersByTimeAsync(100);
}

let store: InstanceType<typeof AgentStore>;

beforeEach(async () => {
  vi.useFakeTimers();
  calls.length = 0;
  listeners.clear();
  for (const k of Object.keys(commands)) delete commands[k];
  app.agentStatus = 'disconnected';
  app.currentUuid = null;
  app.pageUnsaved = false;
  commands.daemon_status = () => ({ state: 'connected' });
  commands.agent_list = () => AGENTS;
  commands.approval_list = () => [];
  commands.vault_agent_permission_get = () => 'approve';
  commands.session_list = () => [session('s1')];
  commands.session_events = () => log({ type: 'state', state: 'idle' });
  store = new AgentStore();
  await store.init();
  await settle();
});

afterEach(() => {
  store.dispose();
  vi.useRealTimers();
});

describe('AgentStore', () => {
  it('loads agents, the vault permission and the newest live session on connect', () => {
    expect(store.connection.state).toBe('connected');
    expect(store.agent).toBe('opencode');
    expect(store.permission).toBe('approve');
    expect(store.activeId).toBe('s1');
    expect(app.agentStatus).toBe('connected');
    expect(store.panelView).toBe('ready');
  });

  it('shows the daemon-unavailable view and the hollow mark when the connection drops', () => {
    emit('agent-connection', { state: 'unavailable', message: 'no daemon' });
    expect(store.panelView).toBe('daemon_unavailable');
    expect(store.connection.message).toBe('no daemon');
    expect(app.agentStatus).toBe('disconnected');
  });

  it('reloads after a reconnect', async () => {
    emit('agent-connection', { state: 'unavailable' });
    const before = calls.filter((c) => c.command === 'session_list').length;
    emit('agent-connection', { state: 'connected' });
    await settle();
    expect(store.connection.state).toBe('connected');
    expect(calls.filter((c) => c.command === 'session_list').length).toBe(before + 1);
  });

  it('builds the feed from live events, batching a stream', async () => {
    live('s1', [
      { seq: 2, at: '2026-11-18T10:01:00Z', body: { type: 'user_message', text: 'hi', context: [] } },
      { seq: 3, at: '2026-11-18T10:01:01Z', body: { type: 'state', state: 'working' } },
      { seq: 4, at: '2026-11-18T10:01:02Z', body: { type: 'agent_text', text: 'Hel' } },
      { seq: 5, at: '2026-11-18T10:01:02Z', body: { type: 'agent_text', text: 'lo' } },
    ]);
    await settle();
    expect(store.view.items.map((i) => i.kind)).toEqual(['user', 'agent']);
    expect(store.view.items[1]).toMatchObject({ text: 'Hello' });
    expect(store.working).toBe(true);
    expect(app.agentStatus).toBe('working');
  });

  it('marks the tab at once when an approval arrives, and clears it when answered', () => {
    emit('agent-event', { kind: 'approval_requested', approval: EDIT_APPROVAL });
    expect(app.agentStatus).toBe('approval');
    emit('agent-event', { kind: 'approval_resolved', vault: 'v1', approval: 137, outcome: 'approved' });
    expect(app.agentStatus).toBe('connected');
  });

  it("does not mark the tab for another vault's approval", () => {
    emit('agent-event', { kind: 'approval_requested', approval: { ...EDIT_APPROVAL, vault: 'v2' } });
    expect(app.agentStatus).toBe('connected');
  });

  it('answers an approval through the daemon', async () => {
    commands.approval_answer = () => null;
    await store.answerApproval(137, true);
    expect(calls.at(-1)).toEqual({ command: 'approval_answer', args: { approval: 137, approve: true } });
  });

  it('fetches the missing events when a live one skips ahead', async () => {
    live('s1', [{ seq: 4, at: 'x', body: { type: 'agent_text', text: 'c' } }]);
    await settle();
    const fetches = calls.filter((c) => c.command === 'session_events');
    expect(fetches.at(-1)!.args.afterSeq).toBe(1);
  });

  it('starts a session on the first message when none is live', async () => {
    commands.session_list = () => [];
    store.dispose();
    store = new AgentStore();
    await store.init();
    await settle();
    expect(store.activeId).toBeNull();
    commands.session_create = (args) => ({ ...session('s2', 'starting'), agent: args.agent });
    commands.session_send = () => null;
    commands.session_events = () => [];
    await expect(store.send('hello', [{ path: 'p-vendor', title: 'Vendor onboarding' }])).resolves.toBe(true);
    expect(calls.find((c) => c.command === 'session_create')!.args).toMatchObject({ vault: 'v1', agent: 'opencode' });
    expect(calls.at(-1)).toEqual({
      command: 'session_send',
      args: { session: 's2', text: 'hello', context: [{ path: 'p-vendor', title: 'Vendor onboarding' }] },
    });
  });

  it('changing the agent starts a new session and ends the old one', async () => {
    commands.session_create = (args) => ({ ...session('s2', 'idle'), agent: args.agent });
    commands.session_end = () => null;
    commands.session_events = () => [];
    await store.chooseAgent('claude');
    expect(store.activeId).toBe('s2');
    expect(calls.some((c) => c.command === 'session_end' && c.args.session === 's1')).toBe(true);
  });

  it('shows a failed send as an error', async () => {
    commands.session_send = () => new Error('the session ended');
    await expect(store.send('hello', [])).resolves.toBe(false);
    expect(store.error).toBe('the session ended');
  });

  it('refreshes the open page when the agent changed it and it is clean', async () => {
    app.currentUuid = 'p-vendor';
    app.currentContent = 'old';
    commands.page_read = () => 'new';
    live('s1', [{ seq: 2, at: 'x', body: { type: 'page_changed', path: 'p-vendor' } }]);
    await settle();
    expect(app.currentContent).toBe('new');
  });

  it('leaves the open page alone while it has unsaved edits', async () => {
    app.currentUuid = 'p-vendor';
    app.currentContent = 'old';
    app.pageUnsaved = true;
    commands.page_read = () => 'new';
    live('s1', [{ seq: 2, at: 'x', body: { type: 'page_changed', path: 'p-vendor' } }]);
    await settle();
    expect(app.currentContent).toBe('old');
  });

  it('sets the permission through the registry call', async () => {
    commands.vault_agent_permission_set = (args) => args.level;
    await store.setPermission('auto');
    expect(store.permission).toBe('auto');
    expect(calls.at(-1)).toEqual({ command: 'vault_agent_permission_set', args: { vaultId: 'v1', level: 'auto' } });
  });

  it('keeps the open session on reconnect and refetches only after its last seq', async () => {
    emit('agent-connection', { state: 'unavailable' });
    commands.session_events = (args) => (args.afterSeq === 1 ? log({ type: 'state', state: 'idle' }).map((e) => ({ ...e, seq: 2 })) : []);
    const before = calls.filter((c) => c.command === 'session_events').length;
    emit('agent-connection', { state: 'connected' });
    await settle();
    expect(store.activeId).toBe('s1');
    const fetches = calls.filter((c) => c.command === 'session_events').slice(before);
    expect(fetches.map((c) => c.args.afterSeq)).toEqual([1]);
    expect(store.view.lastSeq).toBe(2);
  });

  it('resyncs on a resync event: events after the last seq and the approval list', async () => {
    commands.approval_list = () => [EDIT_APPROVAL];
    calls.length = 0;
    emit('agent-event', { kind: 'resync' });
    await settle();
    expect(calls.some((c) => c.command === 'approval_list')).toBe(true);
    expect(calls.find((c) => c.command === 'session_events')!.args.afterSeq).toBe(1);
    expect(store.activeId).toBe('s1');
    expect(store.pendingIds.has(137)).toBe(true);
  });

  it('prefers the listed state when the session ended, even if the feed says working', async () => {
    live('s1', [{ seq: 2, at: 'x', body: { type: 'state', state: 'working' } }]);
    await settle();
    expect(store.working).toBe(true);
    commands.session_list = () => [session('s1', 'ended')];
    await store.refreshSessions();
    expect(store.sessionState).toBe('ended');
    expect(store.working).toBe(false);
  });
});
