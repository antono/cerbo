import { describe, expect, it } from 'vitest';
import {
  EXAMPLE_QUERY,
  compactIri,
  errorMessage,
  mapResponse,
  objectUuid,
  rowCountLabel,
  type SparqlResponse,
} from './sparql-console';

const prefixes = {
  cerbo: 'cerbo://ontology/',
  schema: 'https://schema.org/',
  xsd: 'http://www.w3.org/2001/XMLSchema#',
  ex: 'https://example.org/',
  exdeep: 'https://example.org/deep/',
};

const PAGE = '10270714-7005-4627-8b5e-ac75a30c1990';
const GHOST = 'f7e435db-9740-4a2a-8e57-b439c4f8bb18';
const pages = [{ uuid: PAGE, title: 'Berlin' }];

describe('sparql-console IRI compaction', () => {
  it('shows a schema.org IRI in prefixed form', () => {
    expect(compactIri('https://schema.org/Place', prefixes)).toBe('schema:Place');
    expect(compactIri('http://www.w3.org/2001/XMLSchema#integer', prefixes)).toBe('xsd:integer');
  });

  it('prefers the longest matching namespace', () => {
    expect(compactIri('https://example.org/deep/Thing', prefixes)).toBe('exdeep:Thing');
  });

  it('keeps IRIs without a matching prefix or with a path remainder', () => {
    expect(compactIri('https://other.org/x', prefixes)).toBe('https://other.org/x');
    expect(compactIri('https://example.org/a/b', prefixes)).toBe('https://example.org/a/b');
    expect(compactIri('https://schema.org/Place', null)).toBe('https://schema.org/Place');
  });

  it('recognises object IRIs only in the cerbo://objects/<uuid> form', () => {
    expect(objectUuid(`cerbo://objects/${PAGE}`)).toBe(PAGE);
    expect(objectUuid(`cerbo://objects/${PAGE.toUpperCase()}`)).toBe(PAGE);
    expect(objectUuid(`cerbo://${PAGE}`)).toBeNull();
    expect(objectUuid(`cerbo://objects/${PAGE}/x`)).toBeNull();
  });
});

describe('sparql-console result mapping', () => {
  it('maps SELECT bindings to a table with page links and prefixed IRIs', () => {
    const response: SparqlResponse = {
      form: 'results',
      truncated: false,
      limit: 1000,
      json: {
        head: { vars: ['p', 't', 'type'] },
        results: {
          bindings: [
            {
              p: { type: 'uri', value: `cerbo://objects/${PAGE}` },
              t: { type: 'literal', value: 'Berlin', 'xml:lang': 'en' },
              type: { type: 'uri', value: 'https://schema.org/Place' },
            },
            {
              p: { type: 'uri', value: `cerbo://objects/${GHOST}` },
              t: { type: 'literal', value: '137', datatype: 'http://www.w3.org/2001/XMLSchema#integer' },
            },
          ],
        },
      },
    };

    const result = mapResponse(response, pages, prefixes);
    expect(result.kind).toBe('table');
    if (result.kind !== 'table') return;
    expect(result.vars).toEqual(['p', 't', 'type']);
    expect(result.truncated).toBe(false);
    expect(result.rows[0]).toEqual([
      { kind: 'page', uuid: PAGE, title: 'Berlin', iri: `cerbo://objects/${PAGE}` },
      { kind: 'literal', text: 'Berlin', hint: '@en' },
      { kind: 'iri', text: 'schema:Place', iri: 'https://schema.org/Place' },
    ]);
    // An object that is not a live page keeps its raw IRI; unbound is empty.
    expect(result.rows[1]).toEqual([
      { kind: 'iri', text: `cerbo://objects/${GHOST}`, iri: `cerbo://objects/${GHOST}` },
      { kind: 'literal', text: '137', hint: 'xsd:integer' },
      { kind: 'empty' },
    ]);
  });

  it('keeps the truncated flag, the backend limit and a SELECT with no rows', () => {
    const result = mapResponse(
      { form: 'results', truncated: true, limit: 137, json: { head: { vars: ['s'] }, results: { bindings: [] } } },
      pages,
      prefixes,
    );
    expect(result).toEqual({ kind: 'table', vars: ['s'], rows: [], truncated: true, limit: 137 });
  });

  it('maps ASK to a boolean, including false', () => {
    expect(mapResponse({ form: 'results', truncated: false, limit: 1000, json: { head: {}, boolean: false } }, pages, prefixes)).toEqual({
      kind: 'boolean',
      value: false,
    });
  });

  it('passes CONSTRUCT Turtle through', () => {
    const turtle = '<cerbo://objects/x> a <https://schema.org/Place> .\n';
    expect(mapResponse({ form: 'graph', turtle, truncated: true, limit: 137 }, pages, prefixes)).toEqual({
      kind: 'turtle',
      text: turtle,
      truncated: true,
      limit: 137,
    });
  });

  it('maps blank nodes', () => {
    const result = mapResponse(
      {
        form: 'results',
        truncated: false,
        limit: 1000,
        json: { head: { vars: ['b'] }, results: { bindings: [{ b: { type: 'bnode', value: 'b137' } }] } },
      },
      pages,
      prefixes,
    );
    expect(result.kind === 'table' && result.rows[0][0]).toEqual({ kind: 'bnode', text: '_:b137' });
  });
});

describe('sparql-console messages', () => {
  it('keeps the parser position of a syntax error', () => {
    const message = 'SPARQL syntax error: line 1 column 137';
    expect(errorMessage({ kind: 'querySyntax', message })).toBe(message);
  });

  it('says that only read-only queries are allowed', () => {
    expect(errorMessage({ kind: 'readOnlyQuery', message: 'x' })).toMatch(/read-only/);
  });

  it('explains a busy index and falls back to the raw error', () => {
    expect(errorMessage({ kind: 'indexBusy' })).toMatch(/busy/);
    expect(errorMessage('boom')).toBe('boom');
  });

  it('counts rows and has an example query listing pages and titles', () => {
    expect(rowCountLabel(137)).toBe('137 rows');
    expect(rowCountLabel(1)).toBe('1 row');
    expect(EXAMPLE_QUERY).toMatch(/schema:name/);
  });
});
