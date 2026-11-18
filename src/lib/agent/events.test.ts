import { describe, expect, it } from 'vitest';
import { ageLabel, applyApprovalEvent, pendingForVault, pickActiveSession, touchSession } from './events';
import { EDIT_APPROVAL, TOOL_APPROVAL } from './fixtures';
import type { SessionInfo } from './types';

const session = (id: string, state: SessionInfo['state']): SessionInfo => ({
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

describe('applyApprovalEvent', () => {
  it('adds a request once and drops it when resolved', () => {
    const asked = applyApprovalEvent([], { kind: 'approval_requested', approval: EDIT_APPROVAL });
    expect(applyApprovalEvent(asked, { kind: 'approval_requested', approval: EDIT_APPROVAL })).toHaveLength(1);
    const done = applyApprovalEvent(asked, { kind: 'approval_resolved', vault: 'v1', approval: 137, outcome: 'approved' });
    expect(done).toEqual([]);
  });

  it('follows the approval events inside the session feed too', () => {
    const at = '2026-11-18T10:00:00Z';
    const asked = applyApprovalEvent([], {
      kind: 'session',
      vault: 'v1',
      session: 's1',
      event: { seq: 1, at, body: { type: 'approval_requested', approval: TOOL_APPROVAL } },
    });
    expect(asked.map((a) => a.id)).toEqual([138]);
    const done = applyApprovalEvent(asked, {
      kind: 'session',
      vault: 'v1',
      session: 's1',
      event: { seq: 2, at, body: { type: 'approval_resolved', approval_id: 138, outcome: 'timed_out' } },
    });
    expect(done).toEqual([]);
  });

  it('keeps the list on a resync: it is read again, not applied', () => {
    const list = [EDIT_APPROVAL];
    expect(applyApprovalEvent(list, { kind: 'resync' })).toEqual(list);
  });

  it('leaves the list alone for other session events', () => {
    const list = [EDIT_APPROVAL];
    const next = applyApprovalEvent(list, {
      kind: 'session',
      vault: 'v1',
      session: 's1',
      event: { seq: 3, at: 'x', body: { type: 'agent_text', text: 'hi' } },
    });
    expect(next).toEqual(list);
  });
});

describe('pendingForVault', () => {
  it('counts only the open vault', () => {
    expect(pendingForVault([EDIT_APPROVAL, { ...TOOL_APPROVAL, vault: 'v2' }], 'v1').map((a) => a.id)).toEqual([137]);
    expect(pendingForVault([EDIT_APPROVAL], null)).toEqual([]);
  });
});

describe('sessions', () => {
  it('picks the newest live session', () => {
    expect(pickActiveSession([session('c', 'ended'), session('b', 'idle'), session('a', 'working')])).toBe('b');
    expect(pickActiveSession([session('c', 'failed')])).toBeNull();
  });

  it('updates the state and time of one session', () => {
    const next = touchSession([session('a', 'idle'), session('b', 'idle')], 'a', { state: 'working', at: '2026-11-18T11:00:00Z' });
    expect(next[0]).toMatchObject({ state: 'working', updated_at: '2026-11-18T11:00:00Z' });
    expect(next[1].state).toBe('idle');
  });
});

describe('ageLabel', () => {
  const now = Date.parse('2026-11-18T12:00:00Z');
  it('is short', () => {
    expect(ageLabel('2026-11-18T11:59:50Z', now)).toBe('now');
    expect(ageLabel('2026-11-18T11:55:00Z', now)).toBe('5 min');
    expect(ageLabel('2026-11-18T09:00:00Z', now)).toBe('3 h');
    expect(ageLabel('2026-11-16T12:00:00Z', now)).toBe('2 d');
    expect(ageLabel('garbage', now)).toBe('');
  });
});
