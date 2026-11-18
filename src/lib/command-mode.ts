/**
 * Command mode of page search (spec `command-palette`, design D5).
 *
 * A query that starts with `>` lists registry actions instead of pages. This
 * module is pure apart from the in-memory "recently run" list, so the matching
 * rules are testable without the component.
 */

import { ACTIONS, actionAvailable, actionLabel, type Action, type ActionContext } from './actions';

/** The prefix that switches page search into command mode. */
export const COMMAND_PREFIX = '>';

/** True when the search input asks for commands rather than pages. */
export function isCommandQuery(query: string): boolean {
  return query.startsWith(COMMAND_PREFIX);
}

/** The action query after the `>`, trimmed and lower-cased. */
export function commandTerm(query: string): string {
  return query.slice(COMMAND_PREFIX.length).trim().toLowerCase();
}

// Most recent first. In memory only: design D5 does not persist it.
let recent: string[] = [];

/** Remember that `id` ran, so an empty `>` query lists it first. */
export function noteActionRun(id: string): void {
  recent = [id, ...recent.filter((r) => r !== id)];
}

/** Action ids in the order they were last run, most recent first. */
export function recentActionIds(): readonly string[] {
  return recent;
}

/** Forget the recently run list (tests). */
export function clearRecentActions(): void {
  recent = [];
}

/**
 * How well `action` matches `term`: 2 when its label contains it, 1 when only
 * a synonym does, 0 when nothing does. An empty term matches everything.
 */
export function matchRank(action: Action, ctx: ActionContext, term: string): number {
  if (!term) return 2;
  if (actionLabel(action, ctx).toLowerCase().includes(term)) return 2;
  if (action.synonyms?.some((s) => s.toLowerCase().includes(term))) return 1;
  return 0;
}

/**
 * The actions to list for `query` (with or without its leading `>`): those
 * that apply right now and match, label matches before synonym-only matches,
 * recently run actions first within each, then registry order.
 */
export function commandResults(
  query: string,
  ctx: ActionContext,
  actions: readonly Action[] = ACTIONS,
  recentIds: readonly string[] = recent,
): Action[] {
  const term = isCommandQuery(query) ? commandTerm(query) : query.trim().toLowerCase();
  const recency = (a: Action) => {
    const i = recentIds.indexOf(a.id);
    return i === -1 ? Number.MAX_SAFE_INTEGER : i;
  };
  return actions
    .map((action, order) => ({ action, order, rank: matchRank(action, ctx, term) }))
    .filter(({ action, rank }) => rank > 0 && !action.hideInCommands && actionAvailable(action, ctx))
    .sort((a, b) => b.rank - a.rank || recency(a.action) - recency(b.action) || a.order - b.order)
    .map(({ action }) => action);
}

/**
 * Run a chosen action the way command mode does: close search first (so a
 * dialog the action opens is not closed with it), then run it and remember it.
 */
export function runCommand(action: Action, ctx: ActionContext, closeSearch: () => void): void {
  closeSearch();
  noteActionRun(action.id);
  action.run(ctx);
}
