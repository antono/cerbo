import { describe, expect, it } from 'vitest';
import { hotkeySignatures } from './actions';
import {
  badgeText,
  DEFAULT_RIGHT_PANEL,
  parseRightPanel,
  RIGHT_PANELS,
  tabBadge,
  tabForKey,
  tabShortcutOutcome,
  type AgentStatus,
} from './right-panels';

describe('RIGHT_PANELS', () => {
  it('lists Attachments, Links, Outline, Agent in this order', () => {
    expect(RIGHT_PANELS.map((p) => p.id)).toEqual(['attachments', 'links', 'outline', 'agent']);
  });

  it('gives every tab a distinct hotkey and an icon', () => {
    const signatures = RIGHT_PANELS.flatMap((p) => hotkeySignatures(p.hotkey));
    expect(new Set(signatures).size).toBe(RIGHT_PANELS.length);
    for (const p of RIGHT_PANELS) expect(p.icon).toBeTruthy();
  });
});

describe('parseRightPanel', () => {
  it('keeps a known tab and falls back to Links for anything else', () => {
    expect(parseRightPanel('outline')).toBe('outline');
    expect(parseRightPanel('attachments')).toBe('attachments');
    expect(parseRightPanel('similar')).toBe(DEFAULT_RIGHT_PANEL);
    expect(parseRightPanel('')).toBe('links');
    expect(parseRightPanel(null)).toBe('links');
    expect(parseRightPanel(undefined)).toBe('links');
  });
});

describe('tabBadge', () => {
  const state = (attachmentCount: number, agentStatus: AgentStatus = 'disconnected') => ({ attachmentCount, agentStatus });

  it('shows the attachment count above zero, whichever tab is selected', () => {
    expect(tabBadge('attachments', state(137))).toMatchObject({ kind: 'count', value: 137 });
    expect(tabBadge('attachments', state(1))?.label).toBe('1 attachment');
    expect(tabBadge('attachments', state(3))?.label).toBe('3 attachments');
  });

  it('shows no count for zero attachments', () => {
    expect(tabBadge('attachments', state(0))).toBeNull();
  });

  it('shows the agent status, even when not connected', () => {
    expect(tabBadge('agent', state(0, 'disconnected'))).toMatchObject({ kind: 'status', status: 'disconnected' });
    expect(tabBadge('agent', state(0, 'approval'))?.label).toBe('Agent waiting for approval');
  });

  it('has no badge on Links and Outline', () => {
    expect(tabBadge('links', state(5))).toBeNull();
    expect(tabBadge('outline', state(5))).toBeNull();
  });
});

describe('badgeText', () => {
  it('caps wide numbers', () => {
    expect(badgeText(3)).toBe('3');
    expect(badgeText(99)).toBe('99');
    expect(badgeText(137)).toBe('99+');
  });
});

describe('tabShortcutOutcome', () => {
  it('shows a hidden panel', () => {
    expect(tabShortcutOutcome('outline', { panelShown: false, selected: 'links' })).toBe('show');
    expect(tabShortcutOutcome('outline', { panelShown: false, selected: 'outline' })).toBe('show');
  });

  it('selects another tab in a shown panel', () => {
    expect(tabShortcutOutcome('outline', { panelShown: true, selected: 'links' })).toBe('select');
  });

  it('hides the panel when the selected tab is used again', () => {
    expect(tabShortcutOutcome('outline', { panelShown: true, selected: 'outline' })).toBe('hide');
  });
});

describe('tabForKey', () => {
  it('moves with the arrows, wrapping, and jumps with Home and End', () => {
    expect(tabForKey('ArrowRight', 'links')).toBe('outline');
    expect(tabForKey('ArrowRight', 'agent')).toBe('attachments');
    expect(tabForKey('ArrowLeft', 'attachments')).toBe('agent');
    expect(tabForKey('Home', 'outline')).toBe('attachments');
    expect(tabForKey('End', 'outline')).toBe('agent');
    expect(tabForKey('x', 'outline')).toBeNull();
  });
});
