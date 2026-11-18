/** The vault's agent permission (spec "Permission control"). */

export type AgentPermission = 'read' | 'auto' | 'approve';

/** Display order: the most restricted first. */
export const PERMISSIONS: readonly AgentPermission[] = ['read', 'approve', 'auto'];

export const PERMISSION_LABELS: Record<AgentPermission, string> = {
  read: 'Read only',
  approve: 'Ask first',
  auto: 'Auto',
};

export const PERMISSION_HINTS: Record<AgentPermission, string> = {
  read: 'The agent can read pages but not change them.',
  approve: 'The agent asks you before each change.',
  auto: 'The agent changes pages without asking.',
};

/** How much a level lets the agent do. */
const RANK: Record<AgentPermission, number> = { read: 0, approve: 1, auto: 2 };

/** Switching to a level that lets the agent do more: read to auto or approve, approve to auto. */
export function isWidening(from: AgentPermission, to: AgentPermission): boolean {
  return RANK[to] > RANK[from];
}

/** The confirmation text for a widening switch. */
export function wideningWarning(to: AgentPermission): string {
  return to === 'auto'
    ? 'Edits will apply without asking. You can review what changed afterwards in the feed.'
    : 'The agent will be able to propose changes to pages. Each change waits for your approval.';
}

/** A stored or received value as a level; anything unknown is `read`, so it never widens access. */
export function parsePermission(value: unknown): AgentPermission {
  return value === 'auto' || value === 'approve' ? value : 'read';
}
