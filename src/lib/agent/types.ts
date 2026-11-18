/**
 * Mirror of the agent-session part of the daemon protocol, field for field.
 * The source of truth is `core/src/rpc/mod.rs` (PROTOCOL_VERSION 4): the wire
 * uses snake_case, so these types do too. Optional Rust fields (`Option`,
 * skipped when none) are optional here. Change both files together.
 */

export type AgentKind = "opencode" | "claude";

export interface AgentInfo {
  agent: AgentKind;
  available: boolean;
  reason?: string;
  path?: string;
  version?: string;
  logged_in?: boolean;
}

export type SessionState = "starting" | "idle" | "working" | "ended" | "failed";

/** A page attached to a message as context. */
export interface PageRef {
  path: string;
  title: string;
}

export interface SessionInfo {
  id: string;
  /** Vault id. */
  vault: string;
  agent: AgentKind;
  title: string;
  state: SessionState;
  /** RFC 3339. */
  created_at: string;
  /** RFC 3339. */
  updated_at: string;
  resumable: boolean;
  sandboxed: boolean;
  warning?: string;
}

export type ApprovalOutcome = "approved" | "rejected" | "timed_out" | "cancelled";

/** A pending approval: a vault change or, with `action: "tool"`, an agent tool. */
export interface ApprovalInfo {
  id: number;
  vault: string;
  vault_name: string;
  agent: string;
  /** `create`, `edit`, `rename`, `move`, `trash`, `link` or `tool`. */
  action: string;
  /** For `tool`: the session that asks. */
  session?: string;
  /** For `tool`: the agent's title for the call. */
  tool_title?: string;
  /** For `tool`: `read`, `edit`, `execute`, `fetch`, ... */
  tool_kind?: string;
  page?: string;
  summary: string;
  diff?: string;
  /** Unix seconds. */
  requested: number;
  /** Unix seconds. */
  deadline: number;
}

export type ResumeHow = "load" | "transcript";

export type SessionEventBody =
  | { type: "user_message"; text: string; context: PageRef[] }
  | { type: "agent_text"; text: string }
  | { type: "agent_thought"; text: string }
  | {
      type: "tool_call";
      id: string;
      title: string;
      kind: string;
      status: string;
      input_summary?: string;
    }
  | { type: "tool_update"; id: string; status: string; output_summary?: string }
  | { type: "approval_requested"; approval: ApprovalInfo }
  | { type: "approval_resolved"; approval_id: number; outcome: ApprovalOutcome }
  | { type: "page_changed"; path: string }
  | { type: "turn_ended"; stop_reason: string }
  | { type: "state"; state: SessionState; reason?: string; stderr_tail?: string }
  | { type: "resumed"; how: ResumeHow }
  | { type: "error"; message: string };

export interface SessionEvent {
  /** From 1, one more per event within a session. */
  seq: number;
  /** RFC 3339. */
  at: string;
  body: SessionEventBody;
}

/** The daemon event that carries a session event (`{"kind":"session",...}`). */
export interface SessionDaemonEvent {
  kind: "session";
  vault: string;
  session: string;
  event: SessionEvent;
}

/**
 * The daemon event sent to a connection that fell behind (`{"kind":"resync"}`):
 * events were dropped for it, so it reads the state again (`session_list`,
 * `session_events` after the last `seq` it holds, `approval_list`).
 */
export interface ResyncDaemonEvent {
  kind: "resync";
}

/** Calls, as `{method, params}`; the answer type is noted per call. */
export type AgentCall =
  | { method: "agent_list"; params: Record<string, never> } // AgentInfo[]
  // SessionInfo, at once, in the `starting` state: `idle` or `failed` follows as a `state` event
  | { method: "session_create"; params: { vault: string; agent: AgentKind; resume_from?: string } }
  | { method: "session_send"; params: { session: string; text: string; context: PageRef[] } } // null
  | { method: "session_cancel"; params: { session: string } } // null
  | { method: "session_end"; params: { session: string } } // null
  | { method: "session_list"; params: { vault?: string } } // SessionInfo[]
  | { method: "session_delete"; params: { session: string } } // null
  | { method: "session_events"; params: { session: string; after_seq?: number; limit: number } }; // SessionEvent[]
