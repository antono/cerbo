/** What the Agent tab shows as a whole, from the connection and the agents. */

import type { AgentStatus } from '../right-panels';
import type { AgentInfo, AgentKind, SessionState } from './types';

export type ConnectionState = 'connecting' | 'connected' | 'unavailable';

export interface Connection {
  state: ConnectionState;
  message?: string;
}

/** The mark on the Agent tab icon. An approval waiting outranks everything but a lost daemon. */
export function deriveAgentStatus(input: {
  connection: ConnectionState;
  pendingApprovals: number;
  sessionState: SessionState | null;
}): AgentStatus {
  if (input.connection !== 'connected') return 'disconnected';
  if (input.pendingApprovals > 0) return 'approval';
  if (input.sessionState === 'working' || input.sessionState === 'starting') return 'working';
  return 'connected';
}

export type PanelView = 'connecting' | 'daemon_unavailable' | 'no_agent' | 'not_logged_in' | 'ready';

/** Which body the panel shows: an empty or error state, or the chat. */
export function panelView(input: {
  connection: ConnectionState;
  /** Null until the first answer of `agent_list`. */
  agents: readonly AgentInfo[] | null;
  selected: AgentKind | null;
}): PanelView {
  if (input.connection === 'unavailable') return 'daemon_unavailable';
  if (input.connection === 'connecting' || input.agents === null) return 'connecting';
  const available = input.agents.filter((a) => a.available);
  if (available.length === 0) return 'no_agent';
  const chosen = available.find((a) => a.agent === input.selected) ?? available[0];
  return chosen.logged_in === false ? 'not_logged_in' : 'ready';
}

export const AGENT_LABELS: Record<AgentKind, string> = {
  opencode: 'OpenCode',
  claude: 'Claude Code',
};

/** The command Cerbo looks for, per agent: what to install. */
export const AGENT_COMMANDS: Record<AgentKind, string> = {
  opencode: 'opencode',
  claude: 'claude-agent-acp',
};

/** How to sign in, per agent. */
export const AGENT_LOGIN_HINTS: Record<AgentKind, string> = {
  opencode: 'opencode auth login',
  claude: 'claude, then /login',
};

/** The agent to start with: the saved choice if available, else the first available. */
export function pickAgent(agents: readonly AgentInfo[], saved: AgentKind | null): AgentKind | null {
  const available = agents.filter((a) => a.available);
  return (available.find((a) => a.agent === saved) ?? available[0])?.agent ?? null;
}
