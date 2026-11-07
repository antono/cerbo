import { describe, expect, it } from 'vitest';
import {
  buildLinkIndex,
  clampSelection,
  createFenceMemo,
  detectTrigger,
  insertionFor,
  moveSelection,
  RESULT_LIMIT,
  searchLinkIndex,
} from './link-picker';

/** Detect at the end of `text` (the caret after the last typed character). */
function at(text: string) {
  return detectTrigger(text, text.length);
}

describe('detectTrigger', () => {
  it('opens on a single bracket in ordinary text', () => {
    expect(at('See [')).toEqual({ kind: 'link', start: 4, query: '' });
    expect(at('See [ber')).toEqual({ kind: 'link', start: 4, query: 'ber' });
    expect(at('[Berlin Wall')).toEqual({ kind: 'link', start: 0, query: 'Berlin Wall' });
  });

  it('keeps the [[ trigger', () => {
    expect(at('See [[Ber')).toEqual({ kind: 'wikilink', start: 4, query: 'Ber' });
    // A list item may still start with a wikilink.
    expect(at('- [[Ber')).toEqual({ kind: 'wikilink', start: 2, query: 'Ber' });
  });

  it('keeps the (cerbo:// trigger', () => {
    const text = 'x [Berlin](cerbo://ber)';
    expect(at(text)).toEqual({ kind: 'cerbo-link', start: 11, query: 'ber' });
    expect(text.slice(11)).toBe('cerbo://ber)');
  });

  it('places the (cerbo:// start after an earlier unclosed [ on the line', () => {
    const text = 'see [x [Berlin](cerbo://ber)';
    const trigger = at(text);
    expect(trigger).toEqual({ kind: 'cerbo-link', start: 16, query: 'ber' });
    expect(text.slice(trigger!.start)).toBe('cerbo://ber)');
    expect(at('`x` [a [B](cerbo://b)')).toEqual({ kind: 'cerbo-link', start: 11, query: 'b' });
    expect(at('`[a](cerbo://b)')).toBeNull();
  });

  it('does not open after ! (image) or \\ (escaped)', () => {
    expect(at('![')).toBeNull();
    expect(at('![alt')).toBeNull();
    expect(at('\\[')).toBeNull();
  });

  it('opens after an escaped backslash', () => {
    // `\\[`: a literal backslash, then a real bracket.
    expect(at('a \\\\[x')).toEqual({ kind: 'link', start: 4, query: 'x' });
    expect(at('a \\\\\\[x')).toBeNull();
  });

  it('does not open after a list marker at line start (task lists)', () => {
    for (const prefix of ['- ', '* ', '+ ', '1. ', '12) ', '  - ', '> - ']) {
      expect(at(`${prefix}[`), prefix).toBeNull();
      expect(at(`${prefix}[x`), prefix).toBeNull();
    }
    expect(at('intro\n- [')).toBeNull();
  });

  it('opens later in a list item', () => {
    expect(at('- [ ] buy [mi')).toEqual({ kind: 'link', start: 10, query: 'mi' });
  });

  it('dismisses on a space right after [ and on ]', () => {
    expect(at('a [ ')).toBeNull();
    expect(at('a [ber]')).toBeNull();
    // A space later in the query is part of a title.
    expect(at('a [New Y')).not.toBeNull();
  });

  it('does not open inside an inline code span', () => {
    expect(at('`code [')).toBeNull();
    expect(at('``a ` [')).toBeNull();
    expect(at('`code` and [')).toEqual({ kind: 'link', start: 11, query: '' });
    expect(at('`[[x')).toBeNull();
  });

  it('does not open inside a fenced code block', () => {
    expect(at('```\nconst a = [')).toBeNull();
    expect(at('~~~ts\nx\na[')).toBeNull();
    expect(at('```ts[')).toBeNull();
    expect(at('```\ncode\n```\ntext [')).toEqual({ kind: 'link', start: 18, query: '' });
    // A shorter closing fence does not close a longer one.
    expect(at('````\n```\nx[')).toBeNull();
    // A fence line with an info string cannot close a block.
    expect(at('```\n```ts\nx [')).toBeNull();
    expect(at('```\n```  \nx [')).toEqual({ kind: 'link', start: 12, query: '' });
  });

  it('remembers the fence check for the caret line', () => {
    const memo = createFenceMemo();
    const inside = '```\nx = [1, 2';
    expect(detectTrigger(inside, inside.length, { fenceMemo: memo })).toBeNull();
    expect(memo.last).toEqual({ lineStart: 4, inFence: true });
    // The memo answers on the same line, even for text it never scanned.
    const typedOn = 'abc\nx = [1, 2, 3';
    expect(detectTrigger(typedOn, typedOn.length, { fenceMemo: memo })).toBeNull();
    // A lookup on another line drops it and scans again.
    const other = '```\nx\n```\nsee [a';
    expect(detectTrigger(other, other.length, { fenceMemo: memo })).toEqual({ kind: 'link', start: 14, query: 'a' });
    expect(memo.last).toEqual({ lineStart: 10, inFence: false });
  });

  it('scans only the current line', () => {
    expect(at('[open\nnext line')).toBeNull();
    const text = 'one [two\nthree [fo';
    expect(detectTrigger(text, 8)).toEqual({ kind: 'link', start: 4, query: 'two' });
  });
});

describe('insertionFor', () => {
  const target = { uuid: 'u-137', title: 'Berlin' };

  it('builds a full link for [ and [[', () => {
    expect(insertionFor('link', target)).toBe('[Berlin](cerbo://objects/u-137)');
    expect(insertionFor('wikilink', target)).toBe('[Berlin](cerbo://objects/u-137)');
  });

  it('completes only the URL for (cerbo://', () => {
    expect(insertionFor('cerbo-link', target)).toBe('cerbo://objects/u-137)');
  });

  it('escapes brackets in the title', () => {
    expect(insertionFor('link', { uuid: 'u', title: 'a [b]' })).toBe('[a \\[b\\]](cerbo://objects/u)');
  });
});

describe('link index', () => {
  const objects = [
    { uuid: '1', title: 'Hamburg', objectType: 'Page' },
    { uuid: '2', title: 'Berlin', objectType: 'Page' },
    { uuid: '3', title: 'Uber trip', objectType: 'Event' },
    { uuid: '4', title: 'photo.png', objectType: 'Attachment' },
    { uuid: '5', title: 'Notes', objectType: 'Page' },
  ];
  const paths: Record<string, string> = { '5': 'cities/hamburg' };
  const index = buildLinkIndex(objects, (uuid) => paths[uuid]);
  const titles = (q: string) => searchLinkIndex(index, q).map((c) => c.title);

  it('lists the first objects sorted by title for an empty query', () => {
    expect(titles('')).toEqual(['Berlin', 'Hamburg', 'Notes', 'photo.png', 'Uber trip']);
  });

  it('matches title, type and virtual path case-insensitively, prefix first', () => {
    expect(titles('ber')).toEqual(['Berlin', 'Uber trip']);
    expect(titles('ATTACH')).toEqual(['photo.png']);
    expect(titles('hamburg')).toEqual(['Hamburg', 'Notes']);
  });

  it('does not match across fields', () => {
    // "trip" + "event" would join as "tripevent" without the separator.
    expect(titles('tripevent')).toEqual([]);
  });

  it('carries the virtual path', () => {
    expect(searchLinkIndex(index, 'notes')[0].virtualPath).toBe('cities/hamburg');
  });

  it('sorts by lowercase title', () => {
    const idx = buildLinkIndex([
      { uuid: 'a', title: 'beta', objectType: 'Page' },
      { uuid: 'b', title: 'Alpha', objectType: 'Page' },
      { uuid: 'c', title: 'alpha', objectType: 'Page' },
    ]);
    expect(searchLinkIndex(idx, '').map((c) => c.uuid)).toEqual(['b', 'c', 'a']);
  });

  it('caps the result list', () => {
    const many = Array.from({ length: 137 }, (_, i) => ({ uuid: `${i}`, title: `Item ${i}`, objectType: 'Page' }));
    const big = buildLinkIndex(many);
    expect(searchLinkIndex(big, '')).toHaveLength(RESULT_LIMIT);
    expect(searchLinkIndex(big, 'item')).toHaveLength(RESULT_LIMIT);
    expect(searchLinkIndex(big, 'page')).toHaveLength(RESULT_LIMIT);
  });

  it('filters 13 700 objects in under 1 ms per query', () => {
    const types = ['Page', 'Attachment', 'Person', 'Place'];
    const words = ['berlin', 'notes', 'meeting', 'project', 'travel', 'recipe', 'garden', 'invoice'];
    const synthetic = Array.from({ length: 13_700 }, (_, i) => ({
      uuid: `00000000-0000-4000-8000-${String(i).padStart(12, '0')}`,
      title: `${words[i % words.length]} ${words[(i * 7) % words.length]} ${i}`,
      objectType: types[i % types.length],
    }));
    const big = buildLinkIndex(synthetic, (uuid) => `area/${uuid.slice(-3)}`);
    const queries = ['b', 'be', 'ber', 'berl', 'zzz', 'att', 'area/13', 'meeting gar', '137'];

    // Warm up the JIT before timing.
    for (let i = 0; i < 20; i++) for (const q of queries) searchLinkIndex(big, q);

    const rounds = 20;
    const t0 = performance.now();
    for (let i = 0; i < rounds; i++) for (const q of queries) searchLinkIndex(big, q);
    const perQuery = (performance.now() - t0) / (rounds * queries.length);
    expect(perQuery).toBeLessThan(1);
    expect(searchLinkIndex(big, 'ber')[0].title.startsWith('berlin')).toBe(true);
  });

  it('builds the index for 13 700 objects within an idle period', () => {
    // The component builds in idle time, off the keystroke path; this bounds
    // the one synchronous build when the picker opens before that ran.
    const synthetic = Array.from({ length: 13_700 }, (_, i) => ({
      uuid: `u-${i}`,
      title: `note ${(i * 7919) % 13_700} ${i % 137}`,
      objectType: 'Page',
    }));
    buildLinkIndex(synthetic); // warm-up
    const t0 = performance.now();
    buildLinkIndex(synthetic, (uuid) => `area/${uuid.slice(-3)}`);
    expect(performance.now() - t0).toBeLessThan(50);
  });
});

describe('selection', () => {
  it('moves with wrap-around', () => {
    expect(moveSelection(0, 1, 3)).toBe(1);
    expect(moveSelection(2, 1, 3)).toBe(0);
    expect(moveSelection(0, -1, 3)).toBe(2);
    expect(moveSelection(0, 1, 1)).toBe(0);
    expect(moveSelection(0, -1, 0)).toBe(0);
  });

  it('stays inside a list that shrank', () => {
    expect(clampSelection(5, 2)).toBe(1);
    expect(clampSelection(1, 2)).toBe(1);
    expect(clampSelection(3, 0)).toBe(0);
    expect(moveSelection(5, 1, 2)).toBe(0);
  });
});
