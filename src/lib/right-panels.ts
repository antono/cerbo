/**
 * The registry of right panel sub-panels (change `add-right-panel-tabs`,
 * spec `desktop-right-panel`, design D1). The tab strip, the hotkeys and the
 * command palette entries are all generated from `RIGHT_PANELS`, so a new
 * sub-panel is one entry here plus its component in the host.
 *
 * Pure: no Svelte components here (the host maps ids to components), so
 * vitest runs it without a DOM.
 */

import { Bot, List, Paperclip, TextQuote } from 'lucide-svelte';
import type { Hotkey } from './actions';

export type RightPanelId = 'attachments' | 'links' | 'outline' | 'agent';

/** What the Agent tab's status mark shows (spec "Tab badges"). */
export type AgentStatus = 'connected' | 'working' | 'approval' | 'disconnected';

export interface RightPanelDef {
  id: RightPanelId;
  /** Tab name: tooltip, accessible name and "Show <label>" command. */
  label: string;
  icon: typeof List;
  /** Layout-independent: `Ctrl+Alt+1` (`Cmd+Alt+1` on Mac) and so on. */
  hotkey: Hotkey;
  /** Which badge the tab carries, if any. */
  badge?: 'count' | 'status';
}

/** Tab order: left to right in the strip. */
export const RIGHT_PANELS: readonly RightPanelDef[] = [
  { id: 'attachments', label: 'Attachments', icon: Paperclip, hotkey: { mod: true, alt: true, code: 'Digit1' }, badge: 'count' },
  { id: 'links', label: 'Links', icon: List, hotkey: { mod: true, alt: true, code: 'Digit2' } },
  { id: 'outline', label: 'Outline', icon: TextQuote, hotkey: { mod: true, alt: true, code: 'Digit3' } },
  { id: 'agent', label: 'Agent', icon: Bot, hotkey: { mod: true, alt: true, code: 'Digit4' }, badge: 'status' },
];

/** The tab shown when nothing valid is saved. */
export const DEFAULT_RIGHT_PANEL: RightPanelId = 'links';

export function rightPanelById(id: RightPanelId): RightPanelDef {
  const panel = RIGHT_PANELS.find((p) => p.id === id);
  if (!panel) throw new Error(`Unknown right panel: ${id}`);
  return panel;
}

/** A saved or incoming value as a tab id; anything unknown is Links. */
export function parseRightPanel(value: unknown): RightPanelId {
  return RIGHT_PANELS.find((p) => p.id === value)?.id ?? DEFAULT_RIGHT_PANEL;
}

/** What a tab badge shows. */
export type TabBadge =
  | { kind: 'count'; value: number; label: string }
  | { kind: 'status'; status: AgentStatus; label: string };

export const AGENT_STATUS_LABELS: Record<AgentStatus, string> = {
  connected: 'Agent connected',
  working: 'Agent working',
  approval: 'Agent waiting for approval',
  disconnected: 'Agent not connected',
};

/** The badge of tab `id`, or null: attachments show a count above zero, the agent its status. */
export function tabBadge(id: RightPanelId, state: { attachmentCount: number; agentStatus: AgentStatus }): TabBadge | null {
  const def = rightPanelById(id);
  if (def.badge === 'count') {
    const n = state.attachmentCount;
    return n > 0 ? { kind: 'count', value: n, label: `${n} ${n === 1 ? 'attachment' : 'attachments'}` } : null;
  }
  if (def.badge === 'status') {
    return { kind: 'status', status: state.agentStatus, label: AGENT_STATUS_LABELS[state.agentStatus] };
  }
  return null;
}

/** The count as drawn in the badge: `99+` instead of a wide number. */
export function badgeText(value: number): string {
  return value > 99 ? '99+' : String(value);
}

/** What a tab's hotkey or command does (spec "Tab hotkeys and commands"). */
export type TabShortcutOutcome = 'show' | 'select' | 'hide';

/**
 * Hidden panel: show it with the tab selected. Shown with another tab: select
 * the tab. Shown with this tab already selected: hide the panel.
 */
export function tabShortcutOutcome(
  id: RightPanelId,
  state: { panelShown: boolean; selected: RightPanelId },
): TabShortcutOutcome {
  if (!state.panelShown) return 'show';
  return state.selected === id ? 'hide' : 'select';
}

/** The next tab in the strip for ArrowLeft/ArrowRight/Home/End, or null for other keys. */
export function tabForKey(key: string, current: RightPanelId): RightPanelId | null {
  const i = RIGHT_PANELS.findIndex((p) => p.id === current);
  const n = RIGHT_PANELS.length;
  switch (key) {
    case 'ArrowRight':
      return RIGHT_PANELS[(i + 1) % n].id;
    case 'ArrowLeft':
      return RIGHT_PANELS[(i - 1 + n) % n].id;
    case 'Home':
      return RIGHT_PANELS[0].id;
    case 'End':
      return RIGHT_PANELS[n - 1].id;
    default:
      return null;
  }
}
