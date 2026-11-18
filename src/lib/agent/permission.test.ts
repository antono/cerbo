import { describe, expect, it } from 'vitest';
import { isWidening, parsePermission, PERMISSIONS, wideningWarning } from './permission';
import { deriveAgentStatus, panelView, pickAgent } from './status';
import type { AgentInfo } from './types';

describe('isWidening', () => {
  it('asks when the agent gets more room', () => {
    expect(isWidening('read', 'auto')).toBe(true);
    expect(isWidening('read', 'approve')).toBe(true);
    expect(isWidening('approve', 'auto')).toBe(true);
  });

  it('does not ask when it gets less or the same', () => {
    expect(isWidening('auto', 'approve')).toBe(false);
    expect(isWidening('auto', 'read')).toBe(false);
    expect(isWidening('approve', 'read')).toBe(false);
    for (const level of PERMISSIONS) expect(isWidening(level, level)).toBe(false);
  });

  it('warns about unattended edits for auto', () => {
    expect(wideningWarning('auto')).toMatch(/without asking/);
    expect(wideningWarning('approve')).toMatch(/approval/);
  });
});

describe('parsePermission', () => {
  it('reads unknown values as read-only', () => {
    expect(parsePermission('auto')).toBe('auto');
    expect(parsePermission('approve')).toBe('approve');
    expect(parsePermission('root')).toBe('read');
    expect(parsePermission(undefined)).toBe('read');
  });
});

describe('deriveAgentStatus', () => {
  it('shows a lost daemon above everything', () => {
    expect(deriveAgentStatus({ connection: 'unavailable', pendingApprovals: 2, sessionState: 'working' })).toBe('disconnected');
    expect(deriveAgentStatus({ connection: 'connecting', pendingApprovals: 0, sessionState: null })).toBe('disconnected');
  });

  it('shows a waiting approval above working', () => {
    expect(deriveAgentStatus({ connection: 'connected', pendingApprovals: 1, sessionState: 'working' })).toBe('approval');
  });

  it('shows working, then connected', () => {
    expect(deriveAgentStatus({ connection: 'connected', pendingApprovals: 0, sessionState: 'working' })).toBe('working');
    expect(deriveAgentStatus({ connection: 'connected', pendingApprovals: 0, sessionState: 'idle' })).toBe('connected');
    expect(deriveAgentStatus({ connection: 'connected', pendingApprovals: 0, sessionState: null })).toBe('connected');
  });
});

const oc: AgentInfo = { agent: 'opencode', available: true, logged_in: true };
const cl: AgentInfo = { agent: 'claude', available: false, reason: 'claude-agent-acp not found' };

describe('panelView', () => {
  it('shows the daemon error with Retry when it is unreachable', () => {
    expect(panelView({ connection: 'unavailable', agents: null, selected: null })).toBe('daemon_unavailable');
  });

  it('waits for the connection and the agent list', () => {
    expect(panelView({ connection: 'connecting', agents: [oc], selected: null })).toBe('connecting');
    expect(panelView({ connection: 'connected', agents: null, selected: null })).toBe('connecting');
  });

  it('explains what to install when no agent is available', () => {
    expect(panelView({ connection: 'connected', agents: [cl, { ...oc, available: false }], selected: null })).toBe('no_agent');
  });

  it('flags an agent that is not signed in', () => {
    expect(panelView({ connection: 'connected', agents: [{ ...oc, logged_in: false }], selected: 'opencode' })).toBe('not_logged_in');
  });

  it('is ready when the agent runs; unknown login counts as ready', () => {
    expect(panelView({ connection: 'connected', agents: [oc, cl], selected: 'opencode' })).toBe('ready');
    expect(panelView({ connection: 'connected', agents: [{ agent: 'opencode', available: true }], selected: null })).toBe('ready');
  });

  it('falls back to an available agent when the selected one is gone', () => {
    expect(panelView({ connection: 'connected', agents: [oc, cl], selected: 'claude' })).toBe('ready');
  });
});

describe('pickAgent', () => {
  it('keeps the saved agent when available, else the first available', () => {
    const both = [oc, { ...cl, available: true }];
    expect(pickAgent(both, 'claude')).toBe('claude');
    expect(pickAgent([oc, cl], 'claude')).toBe('opencode');
    expect(pickAgent([cl], null)).toBeNull();
  });
});
