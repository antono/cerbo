import { describe, expect, it } from 'vitest';
import { applyMention, canSend, composerKey, detectMention, filterMentions, messageContext } from './composer';

const PAGES = [
  { uuid: 'a', title: 'Vendor onboarding', virtualPath: 'work' },
  { uuid: 'b', title: 'Seven wonders', virtualPath: null },
  { uuid: 'c', title: 'Venn diagrams', virtualPath: 'math' },
  { uuid: 'd', title: 'Meeting notes', virtualPath: null },
];

describe('detectMention', () => {
  it('finds the word after an @ at the start or after a space', () => {
    expect(detectMention('@ven', 4)).toEqual({ start: 0, query: 'ven' });
    expect(detectMention('look at @ven', 12)).toEqual({ start: 8, query: 'ven' });
    expect(detectMention('@', 1)).toEqual({ start: 0, query: '' });
  });

  it('ignores an @ inside a word, such as an email address', () => {
    expect(detectMention('mail me@example', 15)).toBeNull();
  });

  it('only looks before the caret', () => {
    expect(detectMention('@vendor rest', 4)).toEqual({ start: 0, query: 'ven' });
  });

  it('ends at a newline or another @', () => {
    expect(detectMention('@ven\nnext', 9)).toBeNull();
    expect(detectMention('@a @b', 5)).toEqual({ start: 3, query: 'b' });
    expect(detectMention('x', 1)).toBeNull();
  });
});

describe('applyMention', () => {
  it('removes the typed query so the page lives in a chip', () => {
    const text = 'see @ven please';
    const trigger = detectMention(text, 8)!;
    expect(applyMention(text, trigger, 8)).toEqual({ text: 'see  please', caret: 4 });
  });
});

describe('filterMentions', () => {
  it('lists pages by prefix first, then by substring', () => {
    expect(filterMentions(PAGES, 'ven').map((c) => c.title)).toEqual([
      'Vendor onboarding',
      'Venn diagrams',
      'Seven wonders',
    ]);
  });

  it('matches the virtual path too', () => {
    expect(filterMentions(PAGES, 'math').map((c) => c.uuid)).toEqual(['c']);
  });

  it('lists some pages for an empty query and respects the limit', () => {
    expect(filterMentions(PAGES, '', [], 2)).toHaveLength(2);
  });

  it('leaves out pages already attached', () => {
    expect(filterMentions(PAGES, 'ven', ['a']).map((c) => c.uuid)).toEqual(['c', 'b']);
  });

  it('finds nothing when nothing matches', () => {
    expect(filterMentions(PAGES, 'zzz')).toEqual([]);
  });
});

describe('messageContext', () => {
  const open = { path: 'a', title: 'Vendor onboarding' };
  const other = { path: 'd', title: 'Meeting notes' };

  it('puts the open page first', () => {
    expect(messageContext({ openPage: open, openPageRemoved: false, mentions: [other] })).toEqual([open, other]);
  });

  it('leaves the open page out once its chip is removed', () => {
    expect(messageContext({ openPage: open, openPageRemoved: true, mentions: [other] })).toEqual([other]);
  });

  it('does not repeat a page that is both open and mentioned', () => {
    expect(messageContext({ openPage: open, openPageRemoved: false, mentions: [open] })).toEqual([open]);
  });

  it('has no context without an open page', () => {
    expect(messageContext({ openPage: null, openPageRemoved: false, mentions: [] })).toEqual([]);
  });
});

describe('composerKey and canSend', () => {
  it('sends on Enter and breaks the line on Shift+Enter', () => {
    expect(composerKey({ key: 'Enter', shiftKey: false })).toBe('send');
    expect(composerKey({ key: 'Enter', shiftKey: true })).toBe('newline');
    expect(composerKey({ key: 'a', shiftKey: false })).toBe('none');
  });

  it('leaves Enter to the input method while composing', () => {
    expect(composerKey({ key: 'Enter', shiftKey: false, isComposing: true })).toBe('none');
  });

  it('does not send empty text or while the agent works', () => {
    expect(canSend('  \n', false)).toBe(false);
    expect(canSend('hi', true)).toBe(false);
    expect(canSend('hi', false)).toBe(true);
  });
});
