/**
 * The Agent tab's feed as a pure function of a session's event list (design
 * D5): replaying a recorded log gives the same feed the live panel showed, so
 * the reducer is tested without an agent or a daemon.
 */

import type { ApprovalInfo, ApprovalOutcome, PageRef, SessionEvent, SessionState } from './types';

export type FeedItem =
  | { kind: 'user'; key: number; text: string; context: PageRef[] }
  | { kind: 'agent'; key: number; text: string }
  | { kind: 'thought'; key: number; text: string }
  | {
      kind: 'tool';
      key: number;
      id: string;
      title: string;
      toolKind: string;
      status: string;
      input?: string;
      output?: string;
    }
  | { kind: 'approval'; key: number; approval: ApprovalInfo; outcome: ApprovalOutcome | null }
  | { kind: 'page'; key: number; path: string }
  | { kind: 'notice'; key: number; level: 'info' | 'error'; text: string; detail?: string };

export interface SessionView {
  items: FeedItem[];
  /** Approvals asked and not yet answered, in order. */
  pending: ApprovalInfo[];
  /** The newest `seq` seen; 0 for no events. */
  lastSeq: number;
  /** The session's last announced state, or null before any. */
  state: SessionState | null;
  /** A turn is running (`working`). */
  working: boolean;
}

/** Tool status as the row shows it. */
export type ToolStatusKind = 'running' | 'done' | 'failed';

export function toolStatusKind(status: string): ToolStatusKind {
  switch (status) {
    case 'completed':
    case 'done':
      return 'done';
    case 'failed':
    case 'error':
    case 'cancelled':
      return 'failed';
    default:
      return 'running';
  }
}

const STOP_NOTICES: Record<string, string> = {
  cancelled: 'Stopped',
  max_tokens: 'The agent stopped: it reached its length limit',
  max_turn_requests: 'The agent stopped: it reached its request limit',
  refusal: 'The agent declined to continue',
};

/** Build the feed from the events of one session, ordered by `seq`. */
export function buildSessionView(events: readonly SessionEvent[]): SessionView {
  const items: FeedItem[] = [];
  const pending = new Map<number, ApprovalInfo>();
  let state: SessionState | null = null;
  let lastSeq = 0;
  // Streaming text joins the item before it; anything else starts a new one.
  let mergeable: 'agent' | 'thought' | null = null;

  const push = (item: FeedItem) => {
    items.push(item);
    mergeable = null;
  };
  /** The agent behind every unanswered approval is gone: close the cards. */
  const cancelPending = () => {
    pending.clear();
    items.forEach((it, i) => {
      if (it.kind === 'approval' && it.outcome === null) items[i] = { ...it, outcome: 'cancelled' };
    });
  };
  const toolIndex = (id: string) => items.findIndex((i) => i.kind === 'tool' && i.id === id);

  for (const { seq, body } of events) {
    lastSeq = Math.max(lastSeq, seq);
    switch (body.type) {
      case 'user_message':
        push({ kind: 'user', key: seq, text: body.text, context: body.context ?? [] });
        break;
      case 'agent_text':
      case 'agent_thought': {
        const kind = body.type === 'agent_text' ? 'agent' : 'thought';
        const last = items[items.length - 1];
        if (mergeable === kind && last && last.kind === kind) {
          items[items.length - 1] = { ...last, text: last.text + body.text };
        } else {
          items.push({ kind, key: seq, text: body.text });
          mergeable = kind;
        }
        break;
      }
      case 'tool_call': {
        const i = toolIndex(body.id);
        const old = i >= 0 ? items[i] : null;
        const row: FeedItem = {
          kind: 'tool',
          key: old ? old.key : seq,
          id: body.id,
          title: body.title,
          toolKind: body.kind,
          status: body.status,
          input: body.input_summary,
          output: old && old.kind === 'tool' ? old.output : undefined,
        };
        if (i >= 0) items[i] = row;
        else push(row);
        break;
      }
      case 'tool_update': {
        const i = toolIndex(body.id);
        const row = i >= 0 ? items[i] : null;
        if (row && row.kind === 'tool') {
          items[i] = { ...row, status: body.status, output: body.output_summary ?? row.output };
        }
        break;
      }
      case 'approval_requested':
        pending.set(body.approval.id, body.approval);
        push({ kind: 'approval', key: seq, approval: body.approval, outcome: null });
        break;
      case 'approval_resolved': {
        pending.delete(body.approval_id);
        const i = items.findIndex((it) => it.kind === 'approval' && it.approval.id === body.approval_id);
        const card = i >= 0 ? items[i] : null;
        if (card && card.kind === 'approval') items[i] = { ...card, outcome: body.outcome };
        break;
      }
      case 'page_changed': {
        // One link per page and turn: an agent that edits a page twice shows one row.
        let seen = false;
        for (let i = items.length - 1; i >= 0 && items[i].kind !== 'user'; i--) {
          const it = items[i];
          if (it.kind === 'page' && it.path === body.path) seen = true;
        }
        if (!seen) push({ kind: 'page', key: seq, path: body.path });
        break;
      }
      case 'turn_ended': {
        mergeable = null;
        const text = STOP_NOTICES[body.stop_reason];
        if (text) push({ kind: 'notice', key: seq, level: 'info', text });
        break;
      }
      case 'state':
        state = body.state;
        mergeable = null;
        if (body.state === 'ended' || body.state === 'failed') cancelPending();
        if (body.state === 'failed') {
          push({
            kind: 'notice',
            key: seq,
            level: 'error',
            text: body.reason ? `The agent stopped: ${body.reason}` : 'The agent stopped unexpectedly',
            detail: body.stderr_tail,
          });
        } else if (body.state === 'ended') {
          push({
            kind: 'notice',
            key: seq,
            level: 'info',
            text: body.reason ? `Session ended: ${body.reason}` : 'Session ended',
          });
        }
        break;
      case 'resumed':
        // A resumed session starts without the approvals the old process held.
        cancelPending();
        push({
          kind: 'notice',
          key: seq,
          level: 'info',
          text: body.how === 'load' ? 'Session resumed' : 'Session resumed from its transcript',
        });
        break;
      case 'error':
        push({ kind: 'notice', key: seq, level: 'error', text: body.message });
        break;
      // An event type from a newer daemon is skipped, not fatal.
    }
  }

  return { items, pending: [...pending.values()], lastSeq, state, working: state === 'working' };
}

/**
 * Add `incoming` to `existing`: drops events already held and keeps `seq`
 * order. Live events and a replay may overlap, so the store merges instead of
 * appending. `gap` says an event is missing inside the result, so the caller
 * should fetch the events after the last contiguous one.
 */
export function mergeEvents(
  existing: readonly SessionEvent[],
  incoming: readonly SessionEvent[],
): { events: SessionEvent[]; gap: boolean } {
  const known = new Set(existing.map((e) => e.seq));
  const fresh = incoming.filter((e, i) => !known.has(e.seq) && incoming.findIndex((o) => o.seq === e.seq) === i);
  if (fresh.length === 0) return { events: existing as SessionEvent[], gap: false };
  const events = [...existing, ...fresh].sort((a, b) => a.seq - b.seq);
  let gap = false;
  for (let i = 1; i < events.length; i++) {
    if (events[i].seq !== events[i - 1].seq + 1) gap = true;
  }
  return { events, gap };
}

/** How many feed items render at most; older ones sit behind "Show earlier". */
export const FEED_WINDOW = 150;

/** The newest `limit` items, and how many older ones are held back. */
export function windowItems<T>(items: readonly T[], limit: number): { shown: T[]; hidden: number } {
  if (items.length <= limit) return { shown: items.slice(), hidden: 0 };
  return { shown: items.slice(items.length - limit), hidden: items.length - limit };
}

/** Longest text a message shows before "Show more"; bigger chunks would stall layout. */
export const TEXT_LIMIT = 4000;

/** `text` cut to `limit` characters (at a code point), and whether it was cut. */
export function clipText(text: string, limit = TEXT_LIMIT): { text: string; clipped: boolean } {
  if (text.length <= limit) return { text, clipped: false };
  let end = limit;
  const c = text.charCodeAt(end - 1);
  if (c >= 0xd800 && c <= 0xdbff) end--; // do not split a surrogate pair
  return { text: text.slice(0, end), clipped: true };
}

export type DiffLineKind = 'add' | 'remove' | 'hunk' | 'context';

/** The lines of a unified diff with their kind, for colouring. */
export function diffLines(diff: string): { kind: DiffLineKind; text: string }[] {
  return diff
    .replace(/\n$/, '')
    .split('\n')
    .map((text) => {
      let kind: DiffLineKind = 'context';
      if (text.startsWith('@@')) kind = 'hunk';
      else if (text.startsWith('+') && !text.startsWith('+++')) kind = 'add';
      else if (text.startsWith('-') && !text.startsWith('---')) kind = 'remove';
      return { kind, text };
    });
}
