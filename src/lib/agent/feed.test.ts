import { describe, expect, it } from 'vitest';
import { diffLines, mergeEvents, buildSessionView, toolStatusKind, windowItems } from './feed';
import { EDIT_APPROVAL, EDIT_TURN, TOOL_APPROVAL, log } from './fixtures';

describe('buildSessionView', () => {
  it('replays a recorded turn into a feed', () => {
    const view = buildSessionView(EDIT_TURN);
    expect(view.items.map((i) => i.kind)).toEqual(['user', 'thought', 'tool', 'agent', 'approval', 'page']);
    expect(view.lastSeq).toBe(EDIT_TURN.length);
    expect(view.state).toBe('idle');
    expect(view.working).toBe(false);
  });

  it('joins streamed chunks into one message', () => {
    const view = buildSessionView(EDIT_TURN);
    expect(view.items[1]).toMatchObject({ kind: 'thought', text: 'I should look first.' });
    expect(view.items[3]).toMatchObject({ kind: 'agent', text: 'Found the page. Editing it now.' });
  });

  it('shows text as it arrives, mid-stream', () => {
    const view = buildSessionView(log({ type: 'state', state: 'working' }, ), );
    expect(view.working).toBe(true);
    const part = buildSessionView(log({ type: 'agent_text', text: 'Hel' }, ));
    expect(part.items).toEqual([{ kind: 'agent', key: 1, text: 'Hel' }]);
  });

  it('starts a new message after a tool row or a turn end', () => {
    const view = buildSessionView(
      log(
        { type: 'agent_text', text: 'a' },
        { type: 'tool_call', id: 't', title: 'x', kind: 'read', status: 'pending' },
        { type: 'agent_text', text: 'b' },
        { type: 'turn_ended', stop_reason: 'end_turn' },
        { type: 'agent_text', text: 'c' },
      ),
    );
    expect(view.items.map((i) => i.kind === 'agent' && i.text)).toEqual(['a', false, 'b', 'c']);
  });

  it('updates a tool row in place with its result', () => {
    const view = buildSessionView(EDIT_TURN);
    expect(view.items[2]).toMatchObject({
      kind: 'tool',
      title: 'cerbo_search',
      input: 'vendor',
      status: 'completed',
      output: '3 results',
    });
  });

  it('ignores an update for an unknown tool call', () => {
    const view = buildSessionView(log({ type: 'tool_update', id: 'nope', status: 'completed' }));
    expect(view.items).toEqual([]);
  });

  it('keeps an approval pending until it is answered, then shows the outcome', () => {
    const asked = buildSessionView(EDIT_TURN.slice(0, 10));
    expect(asked.pending.map((a) => a.id)).toEqual([137]);
    expect(asked.items[asked.items.length - 1]).toMatchObject({ kind: 'approval', outcome: null });

    const answered = buildSessionView(EDIT_TURN);
    expect(answered.pending).toEqual([]);
    expect(answered.items[4]).toMatchObject({ kind: 'approval', outcome: 'approved' });
  });

  it('shows a timed-out approval as such', () => {
    const view = buildSessionView(
      log(
        { type: 'approval_requested', approval: EDIT_APPROVAL },
        { type: 'approval_resolved', approval_id: 137, outcome: 'timed_out' },
      ),
    );
    expect(view.items[0]).toMatchObject({ kind: 'approval', outcome: 'timed_out' });
    expect(view.pending).toEqual([]);
  });

  it('tracks several pending approvals, tool approvals included', () => {
    const view = buildSessionView(
      log(
        { type: 'approval_requested', approval: EDIT_APPROVAL },
        { type: 'approval_requested', approval: TOOL_APPROVAL },
        { type: 'approval_resolved', approval_id: 137, outcome: 'rejected' },
      ),
    );
    expect(view.pending.map((a) => a.id)).toEqual([138]);
  });

  it('links a changed page once per turn', () => {
    const view = buildSessionView(EDIT_TURN);
    expect(view.items.filter((i) => i.kind === 'page')).toHaveLength(1);

    const twoTurns = buildSessionView(
      log(
        { type: 'user_message', text: 'a', context: [] },
        { type: 'page_changed', path: 'p' },
        { type: 'user_message', text: 'b', context: [] },
        { type: 'page_changed', path: 'p' },
      ),
    );
    expect(twoTurns.items.filter((i) => i.kind === 'page')).toHaveLength(2);
  });

  it('turns failures into notices with the stderr tail', () => {
    const view = buildSessionView(
      log({ type: 'state', state: 'failed', reason: 'exited with 1', stderr_tail: 'boom' }, { type: 'error', message: 'bad' }),
    );
    expect(view.state).toBe('failed');
    expect(view.items).toEqual([
      { kind: 'notice', key: 1, level: 'error', text: 'The agent stopped: exited with 1', detail: 'boom' },
      { kind: 'notice', key: 2, level: 'error', text: 'bad' },
    ]);
  });

  it('notes a cancelled turn and a resumed session', () => {
    const view = buildSessionView(
      log({ type: 'resumed', how: 'transcript' }, { type: 'turn_ended', stop_reason: 'cancelled' }, { type: 'turn_ended', stop_reason: 'end_turn' }),
    );
    expect(view.items.map((i) => i.kind === 'notice' && i.text)).toEqual([
      'Session resumed from its transcript',
      'Stopped',
    ]);
  });

  it('skips an event type it does not know', () => {
    const odd = { seq: 1, at: 'x', body: { type: 'from_the_future' } } as never;
    expect(buildSessionView([odd]).items).toEqual([]);
  });

  it('is deterministic: replaying a prefix then the rest equals the whole', () => {
    expect(buildSessionView(EDIT_TURN)).toEqual(buildSessionView([...EDIT_TURN]));
  });
});

describe('mergeEvents', () => {
  it('drops events already held and keeps order', () => {
    const { events, gap } = mergeEvents(EDIT_TURN.slice(0, 5), EDIT_TURN.slice(3, 8));
    expect(events.map((e) => e.seq)).toEqual([1, 2, 3, 4, 5, 6, 7, 8]);
    expect(gap).toBe(false);
  });

  it('returns the same array when nothing is new', () => {
    const held = EDIT_TURN.slice(0, 3);
    expect(mergeEvents(held, held.slice(1)).events).toBe(held);
  });

  it('reports a gap when an event is missing', () => {
    const { gap } = mergeEvents(EDIT_TURN.slice(0, 3), EDIT_TURN.slice(5, 7));
    expect(gap).toBe(true);
  });

  it('puts out-of-order live events in order', () => {
    const { events } = mergeEvents([], [EDIT_TURN[2], EDIT_TURN[0], EDIT_TURN[1], EDIT_TURN[0]]);
    expect(events.map((e) => e.seq)).toEqual([1, 2, 3]);
  });
});

describe('windowItems', () => {
  it('shows everything under the limit', () => {
    expect(windowItems([1, 2, 3], 5)).toEqual({ shown: [1, 2, 3], hidden: 0 });
  });
  it('keeps the newest items and counts the rest', () => {
    expect(windowItems([1, 2, 3, 4, 5], 2)).toEqual({ shown: [4, 5], hidden: 3 });
  });
});

describe('toolStatusKind', () => {
  it('maps agent statuses to row states', () => {
    expect(toolStatusKind('pending')).toBe('running');
    expect(toolStatusKind('in_progress')).toBe('running');
    expect(toolStatusKind('completed')).toBe('done');
    expect(toolStatusKind('failed')).toBe('failed');
  });
});

describe('diffLines', () => {
  it('classifies unified diff lines', () => {
    expect(diffLines('--- a\n+++ b\n@@ -1 +1 @@\n x\n-y\n+z\n').map((l) => l.kind)).toEqual([
      'context',
      'context',
      'hunk',
      'context',
      'remove',
      'add',
    ]);
  });
});
