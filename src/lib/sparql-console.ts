/**
 * Pure helpers behind the SPARQL console (change `add-semantic-desktop-ui`,
 * design D4). The `sparql_query` command answers with SPARQL 1.1 Results JSON
 * or Turtle; these turn that answer into what the dialog shows. Kept free of
 * Tauri and Svelte imports so they run under vitest.
 */

import type { PrefixMap } from './mention-plugin';

/** Shown on first open, before any query was run in this app. */
export const EXAMPLE_QUERY = `SELECT ?page ?title
WHERE { ?page schema:name ?title }
ORDER BY ?title`;

/** IRI of a vault object: `cerbo://objects/<uuid>`. */
const OBJECT_IRI = /^cerbo:\/\/objects\/([0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})$/;

/** A local part that reads well after `prefix:` (no path or fragment left). */
const LOCAL_NAME = /^[A-Za-z0-9_][\w.-]*$/;

// ── Wire types (mirror `SparqlResult` in src-tauri/src/semantic.rs) ──────────

/** One term of SPARQL 1.1 Query Results JSON. */
export type SparqlTerm =
  | { type: 'uri'; value: string }
  | { type: 'literal'; value: string; 'xml:lang'?: string; datatype?: string }
  | { type: 'bnode'; value: string }
  | { type: 'triple'; value: unknown };

export interface SparqlResultsJson {
  head: { vars?: string[] };
  results?: { bindings: Record<string, SparqlTerm>[] };
  boolean?: boolean;
}

/** `limit` is the row or triple cap `truncated` refers to (`SPARQL_ROW_LIMIT`). */
export type SparqlResponse =
  | { form: 'results'; json: SparqlResultsJson; truncated: boolean; limit: number }
  | { form: 'graph'; turtle: string; truncated: boolean; limit: number };

/** Mirrors `SemanticCommandError` in src-tauri/src/semantic.rs. */
export type SparqlCommandError =
  | { kind: 'indexBusy' }
  | { kind: 'noEmbeddings' }
  | {
      kind:
        | 'modelUnavailable'
        | 'readOnlyQuery'
        | 'querySyntax'
        | 'queryEvaluation'
        | 'notFound'
        | 'unavailable';
      message: string;
    };

// ── What the dialog shows ────────────────────────────────────────────────────

/** A table cell: a page link, a (possibly prefixed) IRI, a value, or nothing. */
export type ResultCell =
  | { kind: 'page'; uuid: string; title: string; iri: string }
  | { kind: 'iri'; text: string; iri: string }
  | { kind: 'literal'; text: string; hint: string }
  | { kind: 'bnode'; text: string }
  | { kind: 'empty' };

export type ConsoleResult =
  | { kind: 'table'; vars: string[]; rows: ResultCell[][]; truncated: boolean; limit: number }
  | { kind: 'boolean'; value: boolean }
  | { kind: 'turtle'; text: string; truncated: boolean; limit: number };

/** The UUID of a `cerbo://objects/<uuid>` IRI, else null. */
export function objectUuid(iri: string): string | null {
  return OBJECT_IRI.exec(iri)?.[1]?.toLowerCase() ?? null;
}

/**
 * `prefix:local` for an IRI under a known namespace, else the IRI unchanged.
 * The longest matching namespace wins; a remainder that is not a plain name
 * (`/`, `#`, spaces, …) keeps the full IRI, so nothing ambiguous is shown.
 */
export function compactIri(iri: string, prefixes: PrefixMap | null): string {
  if (!prefixes) return iri;
  let best: { prefix: string; namespace: string } | null = null;
  for (const [prefix, namespace] of Object.entries(prefixes)) {
    if (!namespace || !iri.startsWith(namespace)) continue;
    if (!best || namespace.length > best.namespace.length) best = { prefix, namespace };
  }
  if (!best) return iri;
  const local = iri.slice(best.namespace.length);
  if (local !== '' && !LOCAL_NAME.test(local)) return iri;
  return `${best.prefix}:${local}`;
}

/** One cell; `titles` maps live page UUIDs (lower case) to their titles. */
export function mapTerm(
  term: SparqlTerm | undefined,
  titles: ReadonlyMap<string, string>,
  prefixes: PrefixMap | null,
): ResultCell {
  if (!term) return { kind: 'empty' };
  switch (term.type) {
    case 'uri': {
      const uuid = objectUuid(term.value);
      const title = uuid ? titles.get(uuid) : undefined;
      // A deleted or unknown object keeps its raw IRI (design risks).
      if (uuid && title !== undefined) return { kind: 'page', uuid, title, iri: term.value };
      return { kind: 'iri', text: compactIri(term.value, prefixes), iri: term.value };
    }
    case 'literal': {
      const lang = term['xml:lang'];
      const hint = lang ? `@${lang}` : term.datatype ? compactIri(term.datatype, prefixes) : '';
      return { kind: 'literal', text: term.value, hint };
    }
    case 'bnode':
      return { kind: 'bnode', text: `_:${term.value}` };
    default:
      // RDF-star triple terms: shown as their JSON, which is rare enough.
      return { kind: 'literal', text: JSON.stringify(term.value), hint: '' };
  }
}

/** The command's answer as a table, a boolean or Turtle. */
export function mapResponse(
  response: SparqlResponse,
  pages: readonly { uuid: string; title: string }[],
  prefixes: PrefixMap | null,
): ConsoleResult {
  if (response.form === 'graph') {
    return { kind: 'turtle', text: response.turtle, truncated: response.truncated, limit: response.limit };
  }
  const { json } = response;
  if (typeof json.boolean === 'boolean') return { kind: 'boolean', value: json.boolean };

  // One lookup per page, however many cells name it (design D4).
  const titles = new Map(pages.map((p) => [p.uuid.toLowerCase(), p.title]));
  const vars = json.head.vars ?? [];
  const rows = (json.results?.bindings ?? []).map((binding) =>
    vars.map((v) => mapTerm(binding[v], titles, prefixes)),
  );
  return { kind: 'table', vars, rows, truncated: response.truncated, limit: response.limit };
}

/** The message for a rejected `sparql_query` call. */
export function errorMessage(error: unknown): string {
  if (typeof error === 'object' && error !== null && 'kind' in error) {
    const e = error as SparqlCommandError;
    switch (e.kind) {
      case 'indexBusy':
        return 'The index is busy: another Cerbo process is using it. Try again in a moment.';
      case 'readOnlyQuery':
        return 'Only read-only queries are allowed (SELECT, ASK, CONSTRUCT, DESCRIBE).';
      case 'noEmbeddings':
        return 'This vault has no embeddings yet.';
      default:
        // The syntax message carries the parser's line and column.
        return e.message;
    }
  }
  return String(error);
}

/** "137 rows", "1 row". */
export function rowCountLabel(count: number): string {
  return `${count} ${count === 1 ? 'row' : 'rows'}`;
}
