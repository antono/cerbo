/**
 * The Agent tab's state: the connection to the daemon, the agents, the
 * sessions of the open vault and the event list of each loaded session. The
 * feed is built from the event list (`buildSessionView`), so everything here
 * is bookkeeping around that pure function.
 *
 * The store lives as long as the window, not as long as the panel: a request
 * for approval that arrives while the panel is hidden must still mark the tab.
 * `+layout.svelte` starts it with `initAgentStore`.
 */

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { app, loadPages, loadVaultObjects } from '../stores.svelte';
import {
  AGENT_CONNECTION_EVENT,
  AGENT_EVENT,
  applyApprovalEvent,
  isLive,
  pendingForVault,
  pickActiveSession,
  touchSession,
  type AgentDaemonEvent,
  type ConnectionPayload,
} from './events';
import { buildSessionView, mergeEvents, type SessionView } from './feed';
import { parsePermission, type AgentPermission } from './permission';
import { deriveAgentStatus, panelView, pickAgent, type Connection } from './status';
import type { AgentInfo, AgentKind, ApprovalInfo, PageRef, SessionEvent, SessionInfo } from './types';

/** Events asked per `session_events` call. */
const PAGE_SIZE = 500;
/** Live events are applied in batches this far apart, so a fast stream renders once per batch. */
const BATCH_MS = 40;
const AGENT_KEY = 'cerbo.agent.selected';

/** A failed command, with the daemon's code when it came from the daemon. */
export class AgentError extends Error {
  constructor(
    message: string,
    readonly code: string,
  ) {
    super(message);
  }
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (e) {
    if (e && typeof e === 'object' && 'message' in e) {
      const err = e as { message: string; code?: string };
      throw new AgentError(err.message, err.code ?? 'failed');
    }
    throw new AgentError(String(e), 'failed');
  }
}

function savedAgent(): AgentKind | null {
  try {
    const v = localStorage.getItem(AGENT_KEY);
    return v === 'opencode' || v === 'claude' ? v : null;
  } catch {
    return null;
  }
}

function saveAgent(kind: AgentKind): void {
  try {
    localStorage.setItem(AGENT_KEY, kind);
  } catch {
    // A private window or blocked storage: the choice just is not remembered.
  }
}

export class AgentStore {
  connection = $state.raw<Connection>({ state: 'connecting' });
  /** Null until the first `agent_list` answer. */
  agents = $state.raw<AgentInfo[] | null>(null);
  selected = $state.raw<AgentKind | null>(savedAgent());
  sessions = $state.raw<SessionInfo[]>([]);
  activeId = $state.raw<string | null>(null);
  /** Events by session id; only sessions that were opened or are live. */
  eventsBySession = $state.raw<Record<string, SessionEvent[]>>({});
  /** Approvals waiting, of every vault. */
  pending = $state.raw<ApprovalInfo[]>([]);
  permission = $state.raw<AgentPermission>('read');
  /** The last failure to show in the panel, cleared by the next action. */
  error = $state.raw<string | null>(null);
  /** A session is being created or loaded. */
  busy = $state.raw(false);
  loadingEvents = $state.raw(false);

  activeInfo = $derived(this.sessions.find((s) => s.id === this.activeId) ?? null);
  view = $derived.by<SessionView>(() => buildSessionView(this.eventsBySession[this.activeId ?? ''] ?? []));
  /** The agent's state: the list's word when the session is over, else the feed's last, else the list's. */
  sessionState = $derived.by(() => {
    const listed = this.activeInfo?.state ?? null;
    if (listed === 'ended' || listed === 'failed') return listed;
    return this.view.state ?? listed;
  });
  /** Ids of approvals still waiting on the daemon: only these may show buttons. */
  pendingIds = $derived(new Set(this.pending.map((a) => a.id)));
  /** A turn runs: the composer offers Cancel instead of Send. */
  working = $derived(this.sessionState === 'working');
  /** The agent is still starting (creating a session answers before it is ready). */
  starting = $derived(this.sessionState === 'starting');
  vaultPending = $derived(pendingForVault(this.pending, app.activeVaultId));
  panelView = $derived(panelView({ connection: this.connection.state, agents: this.agents, selected: this.selected }));
  /** The agent a new session starts with. */
  agent = $derived(this.agents ? pickAgent(this.agents, this.selected) : null);

  #started = false;
  #unlisten: UnlistenFn[] = [];
  #queue: AgentDaemonEvent[] = [];
  #timer: ReturnType<typeof setTimeout> | null = null;
  #vault: string | null = null;
  /** Bumped whenever the vault changes, so a late answer for the old vault is dropped. */
  #epoch = 0;

  /** Start listening. Safe to call again. */
  async init(): Promise<void> {
    if (this.#started) return;
    this.#started = true;
    this.#unlisten.push(
      await listen<AgentDaemonEvent>(AGENT_EVENT, (e) => this.#enqueue(e.payload)),
      await listen<ConnectionPayload>(AGENT_CONNECTION_EVENT, (e) => this.#setConnection(e.payload)),
    );
    try {
      this.#setConnection(await call<ConnectionPayload>('daemon_status'));
    } catch (e) {
      this.#setConnection({ state: 'unavailable', message: String(e) });
    }
  }

  dispose(): void {
    this.#unlisten.forEach((stop) => stop());
    this.#unlisten = [];
    this.#started = false;
    if (this.#timer) clearTimeout(this.#timer);
    this.#timer = null;
  }

  syncStatus(): void {
    app.agentStatus = deriveAgentStatus({
      connection: this.connection.state,
      pendingApprovals: this.vaultPending.length,
      sessionState: this.sessionState,
    });
  }

  // ── Connection ─────────────────────────────────────────────────────────────

  #setConnection(payload: ConnectionPayload): void {
    const was = this.connection.state;
    this.connection = { state: payload.state, message: payload.message };
    this.syncStatus();
    if (payload.state === 'connected' && was !== 'connected') void this.#onConnected();
  }

  /** Ask the desktop to try the daemon again now (the Retry button). */
  async retry(): Promise<void> {
    this.error = null;
    try {
      this.#setConnection(await call<ConnectionPayload>('daemon_retry'));
    } catch (e) {
      this.error = errorText(e);
    }
  }

  /** The daemon is reachable: load what the panel shows, and catch up on what was missed. */
  async #onConnected(): Promise<void> {
    await this.refreshAgents();
    await this.resync();
  }

  /**
   * Catch up after a reconnect or a `resync` event: the session list, the
   * approvals waiting, and the events of the open session after the last one
   * held. The shown session stays shown while it is still listed.
   */
  async resync(): Promise<void> {
    const vault = this.#vault;
    if (!vault || vault !== app.activeVaultId || this.connection.state !== 'connected') {
      await this.#loadPending();
      await this.setVault(app.activeVaultId, true);
      return;
    }
    const epoch = this.#epoch;
    await this.#loadPending();
    try {
      const sessions = await call<SessionInfo[]>('session_list', { vault });
      if (epoch !== this.#epoch) return;
      this.sessions = sessions;
      const id = this.activeId;
      if (id && sessions.some((s) => s.id === id)) {
        const held = this.eventsBySession[id];
        await this.#loadEvents(id, held?.length ? lastContiguous(held) : 0);
      } else {
        const pick = pickActiveSession(sessions);
        this.activeId = pick;
        if (pick) await this.openSession(pick);
      }
    } catch (e) {
      if (epoch === this.#epoch) this.error = errorText(e);
    }
    this.syncStatus();
  }

  async refreshAgents(): Promise<void> {
    try {
      const agents = await call<AgentInfo[]>('agent_list');
      this.agents = agents;
      const pick = pickAgent(agents, this.selected);
      if (pick !== this.selected) this.selected = pick;
    } catch (e) {
      this.error = errorText(e);
    }
  }

  async #loadPending(): Promise<void> {
    try {
      this.pending = await call<ApprovalInfo[]>('approval_list', { vault: null });
      this.syncStatus();
    } catch (e) {
      this.error = errorText(e);
    }
  }

  // ── Vault and sessions ─────────────────────────────────────────────────────

  /** The open vault changed (or the daemon came back): load its sessions and permission. */
  async setVault(vault: string | null, force = false): Promise<void> {
    if (!force && vault === this.#vault) return;
    this.#vault = vault;
    const epoch = ++this.#epoch;
    this.sessions = [];
    this.activeId = null;
    this.eventsBySession = {};
    this.permission = 'read';
    this.syncStatus();
    if (!vault) return;
    await this.loadPermission(vault, epoch);
    if (this.connection.state !== 'connected') return;
    try {
      const sessions = await call<SessionInfo[]>('session_list', { vault });
      if (epoch !== this.#epoch) return;
      this.sessions = sessions;
      const id = pickActiveSession(sessions);
      if (id) await this.openSession(id);
    } catch (e) {
      if (epoch === this.#epoch) this.error = errorText(e);
    }
    this.syncStatus();
  }

  async loadPermission(vault = this.#vault, epoch = this.#epoch): Promise<void> {
    if (!vault) return;
    try {
      const level = await call<string>('vault_agent_permission_get', { vaultId: vault });
      if (epoch === this.#epoch) this.permission = parsePermission(level);
    } catch (e) {
      if (epoch === this.#epoch) this.error = errorText(e);
    }
  }

  async refreshSessions(): Promise<void> {
    const vault = this.#vault;
    if (!vault) return;
    try {
      const sessions = await call<SessionInfo[]>('session_list', { vault });
      if (vault === this.#vault) this.sessions = sessions;
    } catch (e) {
      this.error = errorText(e);
    }
  }

  /** Show a session, loading its stored feed when it is not held yet. */
  async openSession(id: string): Promise<void> {
    this.error = null;
    this.activeId = id;
    this.syncStatus();
    const held = this.eventsBySession[id];
    await this.#loadEvents(id, held?.length ? lastContiguous(held) : 0);
    this.syncStatus();
  }

  /** Fetch the events of `id` after `after`, page by page, and merge them. */
  async #loadEvents(id: string, after: number): Promise<void> {
    this.loadingEvents = true;
    try {
      let cursor = after;
      for (;;) {
        const page = await call<SessionEvent[]>('session_events', {
          session: id,
          afterSeq: cursor > 0 ? cursor : null,
          limit: PAGE_SIZE,
        });
        this.#merge(id, page);
        if (page.length < PAGE_SIZE) break;
        cursor = page[page.length - 1].seq;
      }
    } catch (e) {
      this.error = errorText(e);
    } finally {
      this.loadingEvents = false;
    }
  }

  #merge(id: string, incoming: SessionEvent[]): void {
    const held = this.eventsBySession[id] ?? [];
    const { events, gap } = mergeEvents(held, incoming);
    if (events !== held) this.eventsBySession = { ...this.eventsBySession, [id]: events };
    if (gap) void this.#loadEvents(id, lastContiguous(events));
  }

  /** Start a new session with the chosen agent; the old live one is ended. */
  async newSession(agent: AgentKind | null = this.agent, resumeFrom?: string): Promise<SessionInfo | null> {
    const vault = this.#vault;
    if (!vault || !agent) return null;
    this.error = null;
    this.busy = true;
    try {
      const previous = this.activeInfo;
      const info = await call<SessionInfo>('session_create', { vault, agent, resumeFrom: resumeFrom ?? null });
      if (vault !== this.#vault) return null;
      this.sessions = [info, ...this.sessions.filter((s) => s.id !== info.id)];
      this.eventsBySession = { ...this.eventsBySession, [info.id]: [] };
      this.activeId = info.id;
      if (previous && previous.id !== info.id && isLive(previous.state)) {
        void call('session_end', { session: previous.id }).catch((e) => (this.error = errorText(e)));
      }
      await this.#loadEvents(info.id, 0);
      return info;
    } catch (e) {
      this.error = errorText(e);
      return null;
    } finally {
      this.busy = false;
      this.syncStatus();
    }
  }

  /** The agent picker: the choice is remembered, and a new session starts with it. */
  async chooseAgent(agent: AgentKind): Promise<void> {
    this.selected = agent;
    saveAgent(agent);
    if (this.activeInfo && this.activeInfo.agent === agent && isLive(this.activeInfo.state)) return;
    await this.newSession(agent);
  }

  async deleteSession(id: string): Promise<void> {
    try {
      await call('session_delete', { session: id });
      this.sessions = this.sessions.filter((s) => s.id !== id);
      const { [id]: _gone, ...rest } = this.eventsBySession;
      this.eventsBySession = rest;
      if (this.activeId === id) this.activeId = pickActiveSession(this.sessions);
    } catch (e) {
      this.error = errorText(e);
    }
    this.syncStatus();
  }

  // ── Messages ───────────────────────────────────────────────────────────────

  /**
   * Send a message. Without a live session one starts first (resuming the
   * shown one when it is resumable).
   */
  async send(text: string, context: PageRef[]): Promise<boolean> {
    this.error = null;
    let info = this.activeInfo;
    if (!info || !isLive(info.state)) {
      info = await this.newSession(this.agent, info?.resumable ? info.id : undefined);
      if (!info) return false;
      if (!(await this.#untilStarted(info.id))) return false;
    }
    try {
      await call('session_send', { session: info.id, text, context });
      return true;
    } catch (e) {
      this.error = errorText(e);
      return false;
    }
  }

  /**
   * Creating a session answers while its agent is still starting, and the
   * daemon refuses a message until it is ready: wait for `idle` (the message
   * stays in the composer meanwhile). The list is read again now and then in
   * case a state event was missed.
   */
  async #untilStarted(id: string, timeoutMs = 60_000): Promise<boolean> {
    const stateOf = () => this.sessions.find((s) => s.id === id)?.state ?? null;
    const view = () => (this.activeId === id ? this.sessionState : stateOf());
    const deadline = Date.now() + timeoutMs;
    let polled = Date.now();
    while (view() === 'starting') {
      if (Date.now() > deadline) {
        this.error = 'the agent did not start in time';
        return false;
      }
      if (Date.now() - polled > 2000) {
        polled = Date.now();
        await this.refreshSessions();
      }
      await new Promise((r) => setTimeout(r, 50));
    }
    const state = view();
    if (state === 'failed' || state === 'ended') {
      this.error = this.error ?? (state === 'failed' ? 'the agent failed to start' : 'the session ended');
      return false;
    }
    return true;
  }

  async cancel(): Promise<void> {
    const id = this.activeId;
    if (!id) return;
    try {
      await call('session_cancel', { session: id });
    } catch (e) {
      this.error = errorText(e);
    }
  }

  async answerApproval(approval: number, approve: boolean): Promise<void> {
    this.error = null;
    try {
      await call('approval_answer', { approval, approve });
    } catch (e) {
      this.error = errorText(e);
    }
  }

  async setPermission(level: AgentPermission): Promise<void> {
    const vault = this.#vault;
    if (!vault) return;
    this.error = null;
    try {
      this.permission = parsePermission(await call<string>('vault_agent_permission_set', { vaultId: vault, level }));
    } catch (e) {
      this.error = errorText(e);
    }
  }

  // ── Live events ────────────────────────────────────────────────────────────

  #enqueue(event: AgentDaemonEvent): void {
    if (event.kind === 'resync') {
      void this.resync();
      return;
    }
    // Approvals are tracked at once: the tab mark must not wait for a batch.
    this.pending = applyApprovalEvent(this.pending, event);
    this.#queue.push(event);
    this.syncStatus();
    if (!this.#timer) this.#timer = setTimeout(() => this.#flush(), BATCH_MS);
  }

  #flush(): void {
    this.#timer = null;
    const batch = this.#queue;
    this.#queue = [];
    const bySession = new Map<string, SessionEvent[]>();
    let unknownSession = false;
    const changedPages = new Set<string>();

    for (const event of batch) {
      if (event.kind !== 'session') continue;
      if (event.vault !== this.#vault) continue;
      const { session } = event;
      if (this.eventsBySession[session] || session === this.activeId) {
        bySession.set(session, [...(bySession.get(session) ?? []), event.event]);
      }
      if (!this.sessions.some((s) => s.id === session)) unknownSession = true;
      const body = event.event.body;
      if (body.type === 'state') {
        this.sessions = touchSession(this.sessions, session, { state: body.state, at: event.event.at });
      } else {
        this.sessions = touchSession(this.sessions, session, { at: event.event.at });
      }
      if (body.type === 'page_changed') changedPages.add(body.path);
    }

    for (const [session, events] of bySession) this.#merge(session, events);
    if (unknownSession) void this.refreshSessions();
    for (const path of changedPages) void this.#refreshChangedPage(path);
    this.syncStatus();
  }

  /** The agent changed a page: show the new text if it is open and clean, and list a new page. */
  async #refreshChangedPage(path: string): Promise<void> {
    if (!app.pages.some((p) => p.uuid === path)) {
      await Promise.all([loadPages(), loadVaultObjects()]);
    }
    if (app.currentUuid !== path || app.pageUnsaved) return;
    try {
      const content = await invoke<string>('page_read', { uuid: path });
      // Look again: the person may have typed while the file was read.
      if (app.currentUuid === path && !app.pageUnsaved && content !== app.currentContent) {
        app.currentContent = content;
      }
    } catch (e) {
      console.error('Failed to refresh the page the agent changed:', e);
    }
  }
}

/** The highest `seq` such that every event from the first held one to it is held. */
function lastContiguous(events: readonly SessionEvent[]): number {
  let last = events[0]?.seq ?? 0;
  for (const e of events) {
    if (e.seq > last + 1) break;
    last = e.seq;
  }
  return last;
}

function errorText(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

export const agentStore = new AgentStore();

/** Start the store with the window; call once. */
export function initAgentStore(): void {
  void agentStore.init();
}
