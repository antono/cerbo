# Command line: `cerbo`

The `cerbo` binary works on vaults from the terminal and from scripts: it is
what the desktop app's launcher and CI use too. This page is a tour; the
complete reference with every flag is the man page, `man cerbo`, whose source
is [cli/man/cerbo.md](../cli/man/cerbo.md).

Every command finds the vault Git-style, by walking up from the current
directory to a `.cerbo/` directory, so `--vault` is rarely needed. Most
commands take `--json` for machine-readable output.

## Create a vault

```bash
cerbo init
```

Creates `.cerbo/` in the current directory, bundles the Schema.org and FOAF
ontologies, and adds `/cerbo/` and `/.cerbo/trash/` to `.gitignore`.

## Pages

```bash
cerbo page create "My First Page"       # prints the new UUID
cerbo page list
cerbo page read  <uuid>
cerbo page write <uuid> "New content"
cerbo page delete <uuid>                # moves the object to .cerbo/trash/
cerbo page backlinks <uuid>             # pages linking here (add --json for an array)
```

Backlinks are answered from the semantic index in `$XDG_CACHE_HOME/cerbo/index/`.
It is built on first use and picks up pages changed outside Cerbo before every
answer, so no `cerbo index` run is needed after a `git pull` or a sync.

## Query with SPARQL

```bash
cerbo sparql 'SELECT ?title WHERE { ?page schema:name ?title } ORDER BY ?title'
cerbo sparql 'ASK { ?page schema:mentions ?x . ?x a schema:Place }'
cerbo sparql --file query.rq --json     # or: echo '<query>' | cerbo sparql -
```

Queries are read-only. `cerbo:`, `schema:` (schema.org), `rdf:`, `rdfs:`,
`xsd:` and the other built-in prefixes, plus the vault's own prefixes, need no
`PREFIX` line. SELECT prints a table, ASK prints `true`/`false`, CONSTRUCT
prints Turtle; `--json` prints SPARQL Results JSON or JSON-LD.

## Search by meaning

```bash
cerbo index --embeddings                       # once per vault: compute embeddings
cerbo search "поездка в Берлин"                # pages about a trip to Berlin, in any language
cerbo similar <uuid> --limit 5
cerbo search "museums" --annotated schema:Place --json
```

Embeddings are computed on this machine with `intfloat/multilingual-e5-small`,
pinned to one revision and checked by SHA-256. The model (about 490 MB) is
downloaded on first use into `$XDG_CACHE_HOME/cerbo/models/`; set
`CERBO_MODEL_DIR` to use a local copy without network access. Pages changed
since their last embedding are embedded again before each search. Pages whose
similarity is at least 0.80 are also linked with `cerbo:similarTo` inside the
index (never in vault files), so SPARQL can query them.

## Rebuild the index

```bash
cerbo index                      # every registered vault, plus the one around the cwd
cerbo index --vault /path/or/id  # one vault
cerbo index --page <uuid>        # one page (incremental)
cerbo index --embeddings         # also refresh embeddings
cerbo index --json               # statistics
```

`cerbo index` regenerates each page's `annotations.ttl` from its content and
rebuilds the semantic index. Run it after bulk imports or editing `page.md`
files by hand; `--page` cannot be combined with `--vault`.

## Symlink tree

```bash
cerbo symlink                    # (re)build <vault>/cerbo/ from the current vault
cerbo symlink --vault /path --json
```

Builds a human-readable tree of relative symlinks named by slug, nested by
`cerbo:virtualPath`, into the UUID store:

```
cerbo/
├── my-notes.md          -> ../.cerbo/objects/<uuid>/page.md
└── research/
    └── rust-ownership.md -> ../../.cerbo/objects/<uuid>/page.md
```

The tree is derived and rebuilt atomically; pages without a `cerbo:slug` are
skipped (run `cerbo index` first to backfill slugs). Details in
[storage.md](storage.md#the-symlink-projection-cerbo).

## Import

```bash
cerbo import https://example.com/article               # a web page as a read-only Source
cerbo import-ontology https://schema.org/version/latest/schemaorg-current-https.jsonld
cerbo import-ontology https://example.org/vocab.ttl --prefix ex --namespace https://example.org/vocab#
```

The last form registers `ex` so that `[Bob]{ex:Person}` resolves; see
[links-and-annotations.md](links-and-annotations.md).

## Vaults

```bash
cerbo vault list                         # registered vaults; (auto) marks discovered ones
cerbo vault add "Work Notes" /path/to/vault
cerbo vault remove <vault-id>
cerbo vault active <vault-id>
cerbo vault approve <vault-id>           # promote an auto-registered vault
```

## Utilities

```bash
cerbo resolve <uuid>     # UUID -> filesystem path
cerbo info               # config dir, cache dir, vaults
cerbo migrate vocabulary --dry-run   # rewrite old-format metadata; see storage.md
```
