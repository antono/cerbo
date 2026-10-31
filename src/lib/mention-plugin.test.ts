import { describe, expect, it } from 'vitest';
import { Carta } from 'carta-md';
import { mentionPlugin, resolveMentionType, type PrefixMap } from './mention-plugin';
import { wikilinkPlugin } from './wikilink-plugin';

const PREFIXES: PrefixMap = {
  schema: 'https://schema.org/',
  cerbo: 'cerbo://ontology/',
};

function render(markdown: string, prefixes: PrefixMap | null = PREFIXES): string {
  const carta = new Carta({
    sanitizer: false,
    extensions: [
      mentionPlugin({ getPrefixes: () => prefixes }),
      wikilinkPlugin({
        getPages: () => ['Berlin'],
        getVaultObjects: () => [],
        onNavigate: () => {},
        onCreate: () => {},
        getVaultPath: () => null,
        getSlug: () => null,
      }),
    ],
  });
  return carta.renderSSR(markdown);
}

/** The text a reader sees, without tags or attribute values. */
function visibleText(html: string): string {
  return html.replace(/<[^>]*>/g, '');
}

describe('resolveMentionType', () => {
  it('resolves a known prefix to a full IRI', () => {
    expect(resolveMentionType(PREFIXES, 'schema', 'Place')).toEqual({
      status: 'resolved',
      written: 'schema:Place',
      iri: 'https://schema.org/Place',
    });
  });

  it('flags an unknown prefix', () => {
    expect(resolveMentionType(PREFIXES, 'zz', 'Widget')).toEqual({
      status: 'unresolved',
      written: 'zz:Widget',
      prefix: 'zz',
    });
  });

  it('does not treat Object.prototype keys as prefixes', () => {
    expect(resolveMentionType(PREFIXES, 'constructor', 'X').status).toBe('unresolved');
  });

  it('stays neutral while the prefix map is not loaded', () => {
    expect(resolveMentionType(null, 'zz', 'Widget').status).toBe('unknown');
  });
});

describe('mentionPlugin', () => {
  it('renders a resolved mention as a span with the type and IRI', () => {
    const html = render('I visited [Berlin]{schema:Place} twice.');
    expect(html).toContain('<span class="mention"');
    expect(html).toContain('data-mention-type="schema:Place"');
    expect(html).toContain('data-mention-iri="https://schema.org/Place"');
    expect(html).toMatch(/title="schema:Place\nhttps:\/\/schema\.org\/Place"/);
    expect(html).not.toContain('<a');
    expect(visibleText(html).trim()).toBe('I visited Berlin twice.');
  });

  it('renders an unknown prefix as an unresolved mention with a warning tooltip', () => {
    const html = render('[Thing]{zz:Widget}');
    expect(html).toContain('class="mention unresolved"');
    expect(html).toContain('Unknown prefix &#x22;zz&#x22;');
    expect(html).not.toContain('data-mention-iri');
    expect(visibleText(html).trim()).toBe('Thing');
  });

  it('renders several mentions in one paragraph', () => {
    const html = render('[Ada]{schema:Person} met [Berlin]{schema:Place} in 137.');
    expect(html.match(/class="mention"/g)).toHaveLength(2);
    expect(visibleText(html).trim()).toBe('Ada met Berlin in 137.');
  });

  it('leaves an adjacent inline link intact', () => {
    const html = render('[Berlin]{schema:Place} and [site](https://example.org/137)');
    expect(html).toContain('<span class="mention"');
    expect(html).toContain('<a href="https://example.org/137">site</a>');
    expect(visibleText(html).trim()).toBe('Berlin and site');
  });

  it('takes back a mention whose text matches a link definition', () => {
    const html = render('[Berlin]{schema:Place} again\n\n[Berlin]: https://example.org/137');
    expect(html).toContain('<span class="mention"');
    expect(html).not.toContain('href="https://example.org/137"');
    expect(visibleText(html).trim()).toBe('Berlin again');
  });

  it('works inside a task list item without touching the checkbox', () => {
    const html = render('- [ ] visit [Berlin]{schema:Place}\n- [x] done');
    expect(html).toContain('type="checkbox"');
    expect(html.match(/type="checkbox"/g)).toHaveLength(2);
    expect(html).toContain('<span class="mention"');
    expect(visibleText(html)).not.toContain('[ ]');
  });

  it('keeps wikilinks next to mentions', () => {
    const html = render('[[Berlin]] is a [Berlin]{schema:Place}');
    expect(html).toContain('data-wikilink-title="Berlin"');
    expect(html).toContain('<span class="mention"');
    expect(visibleText(html).trim()).toBe('Berlin is a Berlin');
  });

  it('never shows braces or the type as text', () => {
    const samples = [
      '[Berlin]{schema:Place}',
      '[Thing]{zz:Widget}',
      '**[Berlin]{schema:Place}**',
      '- [ ] [Berlin]{schema:Place}',
      '[[Berlin]] [Berlin]{schema:Place} [site](https://example.org)',
      '[Berlin]{schema:Place}\n\n[Berlin]: https://example.org',
    ];
    for (const sample of samples) {
      for (const prefixes of [PREFIXES, null]) {
        const text = visibleText(render(sample, prefixes));
        expect(text, sample).not.toMatch(/[{}]/);
        expect(text, sample).not.toMatch(/schema:|zz:/);
      }
    }
  });

  it('leaves code untouched', () => {
    const html = render('`[Berlin]{schema:Place}`');
    expect(html).not.toContain('mention');
    expect(html).toContain('<code>[Berlin]{schema:Place}</code>');
  });

  it('shows backslash-escaped annotation syntax literally', () => {
    const html = render('Write \\[Berlin]{schema:Place} for a mention.');
    expect(html).not.toContain('class="mention');
    expect(visibleText(html).trim()).toBe('Write [Berlin]{schema:Place} for a mention.');
  });

  it('still renders a mention after an escape or a character reference', () => {
    const html = render('\\[x] &amp; &#137; [Berlin]{schema:Place}');
    expect(html.match(/class="mention"/g)).toHaveLength(1);
    expect(visibleText(html)).toContain('[x]');
  });

  it('escapes only the annotation whose bracket is escaped', () => {
    const html = render('\\[Ada]{schema:Person} met [Berlin]{schema:Place}');
    expect(html.match(/class="mention"/g)).toHaveLength(1);
    expect(visibleText(html).trim()).toBe('[Ada]{schema:Person} met Berlin');
  });

  it('ignores bracket syntax without an identifier-shaped prefix', () => {
    const html = render('[a]{b} and [c]{1x:Y}');
    expect(html).not.toContain('mention');
  });
});
