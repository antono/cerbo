/**
 * Recorded session logs as the daemon sends them (`session_events`), for the
 * feed tests and for anything that wants to replay a session without an agent.
 */

import type { ApprovalInfo, SessionEvent, SessionEventBody } from './types';

const AT = '2026-11-18T10:00:00Z';

/** Events numbered from 1 in the order given. */
export function log(...bodies: SessionEventBody[]): SessionEvent[] {
  return bodies.map((body, i) => ({ seq: i + 1, at: AT, body }));
}

export const EDIT_APPROVAL: ApprovalInfo = {
  id: 137,
  vault: 'v1',
  vault_name: 'Notes',
  agent: 'opencode',
  action: 'edit',
  session: 's1',
  page: 'p-vendor',
  summary: 'Edit "Vendor onboarding"',
  diff: '--- a\n+++ b\n@@ -1,2 +1,2 @@\n # Vendor onboarding\n-Draft\n+Final\n',
  requested: 1000,
  deadline: 1300,
};

export const TOOL_APPROVAL: ApprovalInfo = {
  id: 138,
  vault: 'v1',
  vault_name: 'Notes',
  agent: 'claude',
  action: 'tool',
  session: 's1',
  tool_title: 'Run ls',
  tool_kind: 'execute',
  summary: 'Run ls',
  requested: 1000,
  deadline: 1300,
};

/** A turn that searches, streams a reply, asks to edit, is approved and applies the edit. */
export const EDIT_TURN: SessionEvent[] = log(
  { type: 'state', state: 'idle' },
  { type: 'user_message', text: 'Finish the vendor page', context: [{ path: 'p-vendor', title: 'Vendor onboarding' }] },
  { type: 'state', state: 'working' },
  { type: 'agent_thought', text: 'I should ' },
  { type: 'agent_thought', text: 'look first.' },
  { type: 'tool_call', id: 't1', title: 'cerbo_search', kind: 'search', status: 'in_progress', input_summary: 'vendor' },
  { type: 'tool_update', id: 't1', status: 'completed', output_summary: '3 results' },
  { type: 'agent_text', text: 'Found the page. ' },
  { type: 'agent_text', text: 'Editing it now.' },
  { type: 'approval_requested', approval: EDIT_APPROVAL },
  { type: 'approval_resolved', approval_id: 137, outcome: 'approved' },
  { type: 'page_changed', path: 'p-vendor' },
  { type: 'page_changed', path: 'p-vendor' },
  { type: 'turn_ended', stop_reason: 'end_turn' },
  { type: 'state', state: 'idle' },
);
