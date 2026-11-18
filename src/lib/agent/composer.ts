/** Composer logic: the `@` mention, the context chips and the keys. */

import { buildLinkIndex, searchLinkIndex, type LinkCandidate } from '../link-picker';
import type { PageRef } from './types';

export interface MentionTrigger {
  /** Index of the `@`. */
  start: number;
  /** The text typed after it, up to the caret. */
  query: string;
}

/**
 * The `@mention` the caret is in: an `@` at the start or after whitespace and
 * the word typed after it. Null when the caret is not in one, or the text
 * after the `@` holds a newline or a second `@`.
 */
export function detectMention(text: string, caret: number): MentionTrigger | null {
  const before = text.slice(0, caret);
  const at = before.lastIndexOf('@');
  if (at < 0) return null;
  if (at > 0 && !/\s/.test(before[at - 1])) return null;
  const query = before.slice(at + 1);
  if (/[\n@]/.test(query)) return null;
  return { start: at, query };
}

/** Remove the typed `@query` from the text: the chosen page becomes a chip instead. */
export function applyMention(text: string, trigger: MentionTrigger, caret: number): { text: string; caret: number } {
  return { text: text.slice(0, trigger.start) + text.slice(caret), caret: trigger.start };
}

export interface MentionPage {
  uuid: string;
  title: string;
  virtualPath: string | null;
}

export const MENTION_LIMIT = 8;

/** Pages matching `query` by title or path, prefix matches first, minus the ones already attached. */
export function filterMentions(
  pages: readonly MentionPage[],
  query: string,
  attached: readonly string[] = [],
  limit = MENTION_LIMIT,
): LinkCandidate[] {
  const paths = new Map(pages.map((p) => [p.uuid, p.virtualPath]));
  const index = buildLinkIndex(
    pages.map((p) => ({ uuid: p.uuid, title: p.title, objectType: 'page' })),
    (uuid) => paths.get(uuid),
  );
  const skip = new Set(attached);
  return searchLinkIndex(index, query, limit + skip.size)
    .filter((c) => !skip.has(c.uuid))
    .slice(0, limit);
}

/**
 * The context a message carries: the open page unless its chip was removed,
 * then the mentioned pages, without duplicates.
 */
export function messageContext(input: {
  openPage: PageRef | null;
  openPageRemoved: boolean;
  mentions: readonly PageRef[];
}): PageRef[] {
  const out: PageRef[] = [];
  const add = (ref: PageRef) => {
    if (!out.some((r) => r.path === ref.path)) out.push(ref);
  };
  if (input.openPage && !input.openPageRemoved) add(input.openPage);
  input.mentions.forEach(add);
  return out;
}

export type ComposerKey = 'send' | 'newline' | 'none';

/** Enter sends; Shift+Enter is a new line. Enter during IME composition is neither. */
export function composerKey(e: { key: string; shiftKey: boolean; isComposing?: boolean }): ComposerKey {
  if (e.key !== 'Enter' || e.isComposing) return 'none';
  return e.shiftKey ? 'newline' : 'send';
}

/** Whether the message can be sent: some text, and the agent is not busy. */
export function canSend(text: string, working: boolean): boolean {
  return !working && text.trim().length > 0;
}
