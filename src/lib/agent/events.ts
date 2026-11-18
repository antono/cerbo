/**
 * The daemon events the desktop forwards to the webview as the Tauri event
 * `agent-event` (see `src-tauri/src/agent.rs`), and the pure bookkeeping the
 * store does with them.
 */

import type { ApprovalInfo, ApprovalOutcome, SessionDaemonEvent, SessionInfo, SessionState } from './types';

export const AGENT_EVENT = 'agent-event';
export const AGENT_CONNECTION_EVENT = 'agent-connection';

export type AgentDaemonEvent =
  | SessionDaemonEvent
  | { kind: 'approval_requested'; approval: ApprovalInfo }
  | { kind: 'approval_resolved'; vault: string; approval: number; outcome: ApprovalOutcome }
  /** The daemon dropped events for this client: refetch what was missed. */
  | { kind: 'resync' };

/** What `agent-connection` carries. */
export interface ConnectionPayload {
  state: 'connecting' | 'connected' | 'unavailable';
  message?: string;
  daemon_version?: string;
}

/** The list of approvals still waiting, after `event`. Idempotent: a repeat changes nothing. */
export function applyApprovalEvent(pending: readonly ApprovalInfo[], event: AgentDaemonEvent): ApprovalInfo[] {
  switch (event.kind) {
    case 'approval_requested':
      return pending.some((a) => a.id === event.approval.id) ? pending.slice() : [...pending, event.approval];
    case 'approval_resolved':
      return pending.filter((a) => a.id !== event.approval);
    case 'session': {
      const body = event.event.body;
      if (body.type === 'approval_requested') {
        return applyApprovalEvent(pending, { kind: 'approval_requested', approval: body.approval });
      }
      if (body.type === 'approval_resolved') {
        return pending.filter((a) => a.id !== body.approval_id);
      }
      return pending.slice();
    }
    case 'resync':
      return pending.slice();
  }
}

/** Pending approvals that belong to `vault`. */
export function pendingForVault(pending: readonly ApprovalInfo[], vault: string | null): ApprovalInfo[] {
  return vault ? pending.filter((a) => a.vault === vault) : [];
}

/** Whether `info`'s state counts as a session still alive. */
export function isLive(state: SessionState): boolean {
  return state === 'starting' || state === 'idle' || state === 'working';
}

/** The session to show when a vault opens: the newest live one, if any. `sessions` is newest first. */
export function pickActiveSession(sessions: readonly SessionInfo[]): string | null {
  return sessions.find((s) => isLive(s.state))?.id ?? null;
}

/** `sessions` with the state and time of `id` updated by an event. */
export function touchSession(
  sessions: readonly SessionInfo[],
  id: string,
  update: { state?: SessionState; at: string },
): SessionInfo[] {
  return sessions.map((s) =>
    s.id === id ? { ...s, state: update.state ?? s.state, updated_at: update.at } : s,
  );
}

/** A short age for the sessions list: `now`, `5 min`, `3 h`, `2 d`. */
export function ageLabel(iso: string, nowMs: number): string {
  const t = Date.parse(iso);
  if (Number.isNaN(t)) return '';
  const s = Math.max(0, Math.round((nowMs - t) / 1000));
  if (s < 60) return 'now';
  if (s < 3600) return `${Math.floor(s / 60)} min`;
  if (s < 86400) return `${Math.floor(s / 3600)} h`;
  return `${Math.floor(s / 86400)} d`;
}
