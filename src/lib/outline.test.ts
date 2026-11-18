import { describe, expect, it } from 'vitest';
import { createJumpHold, currentHeadingIndex, extractHeadings, headingsFor, outlineDepths, pairRenderedHeadings } from './outline';

describe('pairRenderedHeadings', () => {
  const heads = (items: [number, string][]) => items.map(([level, text], i) => ({ level, text, offset: i, line: i }));

  it('pairs by position when the counts are equal', () => {
    const h = heads([[1, 'A'], [2, 'B']]);
    expect(pairRenderedHeadings(h, [{ level: 1, text: 'x' }, { level: 2, text: 'y' }])).toEqual([
      { heading: 0, rendered: 0 },
      { heading: 1, rendered: 1 },
    ]);
  });

  it('matches by level and text when a raw HTML heading adds one', () => {
    const h = heads([[1, 'Plan'], [2, 'Budget']]);
    const rendered = [
      { level: 1, text: 'Plan' },
      { level: 3, text: 'From raw html' },
      { level: 2, text: 'Budget' },
    ];
    expect(pairRenderedHeadings(h, rendered)).toEqual([
      { heading: 0, rendered: 0 },
      { heading: 1, rendered: 2 },
    ]);
  });

  it('ignores inline markup and takes repeated titles in order', () => {
    const h = heads([[2, '**Notes** on [Berlin](https://x.test)'], [2, 'Notes'], [2, 'Notes']]);
    const rendered = [
      { level: 2, text: 'Notes on Berlin' },
      { level: 2, text: 'Notes' },
      { level: 1, text: 'Extra' },
      { level: 2, text: ' Notes ' },
    ];
    expect(pairRenderedHeadings(h, rendered)).toEqual([
      { heading: 0, rendered: 0 },
      { heading: 1, rendered: 1 },
      { heading: 2, rendered: 3 },
    ]);
  });

  it('leaves out a heading the preview does not show', () => {
    const h = heads([[1, 'Shown'], [2, 'Gone'], [2, 'Last']]);
    const rendered = [{ level: 1, text: 'Shown' }, { level: 2, text: 'Last' }];
    expect(pairRenderedHeadings(h, rendered)).toEqual([
      { heading: 0, rendered: 0 },
      { heading: 2, rendered: 1 },
    ]);
  });
});

describe('extractHeadings', () => {
  it('lists headings in order with level, text, line and offset', () => {
    const md = '# Plan\n\ntext\n\n## Budget\n## Risks\n';
    expect(extractHeadings(md)).toEqual([
      { level: 1, text: 'Plan', offset: 0, line: 0 },
      { level: 2, text: 'Budget', offset: md.indexOf('## Budget'), line: 4 },
      { level: 2, text: 'Risks', offset: md.indexOf('## Risks'), line: 5 },
    ]);
  });

  it('lists setext headings with the level of their underline', () => {
    const md = 'Title\n=====\n\ntext\n\nSection\n-------\nmore\n';
    expect(extractHeadings(md)).toEqual([
      { level: 1, text: 'Title', offset: 0, line: 0 },
      { level: 2, text: 'Section', offset: md.indexOf('Section'), line: 5 },
    ]);
  });

  it('mixes setext and # headings in document order', () => {
    const md = '# One\nTwo\n---\n### Three\nFour\n=\n';
    expect(extractHeadings(md).map((h) => [h.level, h.text])).toEqual([
      [1, 'One'],
      [2, 'Two'],
      [3, 'Three'],
      [1, 'Four'],
    ]);
  });

  it('reads a multi-line paragraph over an underline as one heading', () => {
    const md = 'First line\nsecond line\n===\n';
    expect(extractHeadings(md)).toEqual([{ level: 1, text: 'First line second line', offset: 0, line: 0 }]);
  });

  it('does not take a rule, a list item or an orphan underline for a setext heading', () => {
    expect(extractHeadings('text\n\n---\n\n===\n')).toEqual([]);
    expect(extractHeadings('- item\n---\n> quote\n---\n')).toEqual([]);
  });

  const titles = (md: string) => extractHeadings(md).map((h) => h.text);

  it('takes --- after a list item and its continuation lines for a rule', () => {
    expect(titles('- item\n  continued\n---\n')).toEqual([]);
    expect(titles('- item\ncontinued lazily\n---\n')).toEqual([]);
    expect(titles('1. item\n   more\n===\n')).toEqual([]);
    expect(titles('- item\n\n  second paragraph\n---\n')).toEqual([]);
  });

  it('takes --- after a block quote and its lazy lines for a rule', () => {
    expect(titles('> quote\nlazy line\n---\n')).toEqual([]);
    expect(titles('> quote\n---\n')).toEqual([]);
  });

  it('takes --- after an HTML block for a rule', () => {
    expect(titles('<div>\ntext\n---\n')).toEqual([]);
    expect(titles('<!-- note\nmore\n-->\ntext after\n---\n')).toEqual(['text after']);
    expect(titles('<details>\nsummary\n===\n')).toEqual([]);
    expect(titles('<br>\n---\n')).toEqual([]);
  });

  it('still lists a paragraph with inline HTML, and one after a list or HTML ends', () => {
    expect(titles('<b>Bold</b> title\n---\n')).toEqual(['<b>Bold</b> title']);
    expect(titles('- item\n\nTitle\n---\n')).toEqual(['Title']);
    expect(titles('> quote\n\nTitle\n===\n')).toEqual(['Title']);
    expect(titles('<div>x</div>\n\nTitle\n---\n')).toEqual(['Title']);
    expect(titles('- item\n---\nTitle\n---\n')).toEqual(['Title']);
  });

  it('skips setext look-alikes in fenced code', () => {
    expect(extractHeadings('```\nNot a title\n=====\n```\n')).toEqual([]);
  });

  it('skips front matter, including its closing --- and # comments', () => {
    const md = '---\ntitle: Plan\n# a yaml comment\n---\n# Real\nBody\n---\n';
    expect(extractHeadings(md)).toEqual([
      { level: 1, text: 'Real', offset: md.indexOf('# Real'), line: 4 },
      { level: 2, text: 'Body', offset: md.indexOf('Body'), line: 5 },
    ]);
  });

  it('treats an unclosed leading --- as an ordinary rule', () => {
    expect(extractHeadings('---\n# Real\n')).toEqual([{ level: 1, text: 'Real', offset: 4, line: 1 }]);
  });

  it('returns nothing for an empty page or one without headings', () => {
    expect(extractHeadings('')).toEqual([]);
    expect(extractHeadings('just text\n#not-a-heading\n')).toEqual([]);
  });

  it('skips headings inside fenced code blocks', () => {
    const md = ['# Real', '```sh', '# comment', '```', '~~~', '## also code', '~~~', '## After'].join('\n');
    expect(extractHeadings(md).map((h) => h.text)).toEqual(['Real', 'After']);
  });

  it('keeps a fence open until its own closing fence', () => {
    const md = ['````', '```', '# inside', '````', '# out'].join('\n');
    expect(extractHeadings(md).map((h) => h.text)).toEqual(['out']);
  });

  it('strips closing hashes and trailing spaces, skips empty headings', () => {
    expect(extractHeadings('## Title ##  \n#\n### \n').map((h) => h.text)).toEqual(['Title']);
    expect(extractHeadings('# C#\n').map((h) => h.text)).toEqual(['C#']);
  });

  it('handles Windows line endings and seven hashes', () => {
    const md = '# One\r\n####### seven\r\n## Two\r\n';
    const headings = extractHeadings(md);
    expect(headings.map((h) => h.text)).toEqual(['One', 'Two']);
    expect(headings[1].offset).toBe(md.indexOf('## Two'));
  });
});

describe('headingsFor', () => {
  it('returns the same list for the same text', () => {
    const a = headingsFor('# A\n');
    expect(headingsFor('# A\n')).toBe(a);
    expect(headingsFor('# B\n')).not.toBe(a);
  });
});

describe('outlineDepths', () => {
  it('nests by level and starts at depth 0', () => {
    const h = extractHeadings('## A\n### B\n### C\n## D\n# E\n');
    expect(outlineDepths(h)).toEqual([0, 1, 1, 0, 0]);
  });

  it('skipped levels count as one step', () => {
    expect(outlineDepths(extractHeadings('# A\n#### B\n## C\n'))).toEqual([0, 1, 1]);
  });
});

describe('currentHeadingIndex', () => {
  const tops = [0, 100, 250, 900];
  const at = (edge: number) => currentHeadingIndex(tops.length, (i) => tops[i], edge);

  it('marks the section holding the top of the visible area', () => {
    expect(at(0)).toBe(0);
    expect(at(99)).toBe(0);
    expect(at(100)).toBe(1);
    expect(at(600)).toBe(2);
    expect(at(5000)).toBe(3);
  });

  it('marks nothing above the first heading or without headings', () => {
    expect(currentHeadingIndex(2, (i) => [50, 80][i], 10)).toBe(-1);
    expect(currentHeadingIndex(0, () => 0, 10)).toBe(-1);
  });
});

describe('createJumpHold', () => {
  it('lets a held jump win over the measurement', () => {
    const hold = createJumpHold();
    expect(hold.resolve(0)).toBe(0);
    hold.hold(2);
    expect(hold.resolve(0)).toBe(2);
    expect(hold.resolve(-1)).toBe(2);
  });

  it('follows the measurement again after release', () => {
    const hold = createJumpHold();
    hold.hold(2);
    hold.release();
    expect(hold.resolve(1)).toBe(1);
  });

  it('keeps only the latest jump', () => {
    const hold = createJumpHold();
    hold.hold(2);
    hold.hold(1);
    expect(hold.resolve(0)).toBe(1);
  });
});
