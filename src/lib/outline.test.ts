import { describe, expect, it } from 'vitest';
import { currentHeadingIndex, extractHeadings, headingsFor, outlineDepths } from './outline';

describe('extractHeadings', () => {
  it('lists headings in order with level, text, line and offset', () => {
    const md = '# Plan\n\ntext\n\n## Budget\n## Risks\n';
    expect(extractHeadings(md)).toEqual([
      { level: 1, text: 'Plan', offset: 0, line: 0 },
      { level: 2, text: 'Budget', offset: md.indexOf('## Budget'), line: 4 },
      { level: 2, text: 'Risks', offset: md.indexOf('## Risks'), line: 5 },
    ]);
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
