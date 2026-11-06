---
name: cerbo-cli
description: "Use when the user wants to work with a Cerbo vault from the terminal: create or register a vault, create/read/write/delete pages, link and annotate pages, find backlinks, query the knowledge graph with SPARQL, search pages by meaning, rebuild the index, import a URL or an ontology, build the symlink tree, or run a vocabulary migration. Examples: \"Create a page about X\", \"Which pages link here?\", \"Find notes about Berlin\", \"Reindex the vault\""
---

# Cerbo CLI

`cerbo` is a local-first markdown wiki for humans and agents. Every page is an
object stored by UUID under `<vault>/.cerbo/objects/<uuid>/` (`page.md`,
`meta.ttl`, `annotations.ttl`). Pages link to each other with `cerbo://<uuid>`
and annotate text with HackMD syntax `[Text]{prefix:Type}` (for example
`[Alice]{foaf:Person}`, `[Berlin]{schema:Place}`).

The complete reference is `man cerbo`, written in
[cli/man/cerbo.md](../../../cli/man/cerbo.md). `cerbo <command> --help` is
accurate for every flag. A shorter tour is [docs/cli.md](../../../docs/cli.md).

## Running the binary

Inside this repository there is usually no `cerbo` on `PATH`. Pick one:

```bash
nix build .#cerbo && ./result/bin/cerbo --help      # packaged binary (sets ORT_DYLIB_PATH for search)
nix develop --command cargo run -p cerbo -- --help  # from source, inside the dev shell
```

Below, `cerbo …` stands for whichever of these you use.

## How a vault is found

Every command walks up from the current directory to a `.cerbo/` directory,
like Git. Run commands from inside the vault and `--vault` is never needed.
Resolution order: `--vault` flag, vault around the cwd, `cerbo vault active`,
the single registered vault, else an error. An unregistered vault around the
cwd is auto-registered silently and shows as `(auto)` in `cerbo vault list`.

## Command reference

| Command | What it does |
| --- | --- |
| `cerbo init` | Create `.cerbo/` in the cwd, bundle Schema.org and FOAF, add `/cerbo/` and `/.cerbo/trash/` to `.gitignore` |
| `cerbo vault list\|add\|remove\|active\|approve` | Manage the vault registry (`add <NAME> <PATH>`, the others take a vault ID) |
| `cerbo page list` | Header line, then `<uuid>: <title> [<virtual path>]` per page (`--json`: `uuid`, `title`, `virtualPath` or null); the bundled ontology objects (`Ontology: …`) are listed too |
| `cerbo page create <TITLE>` | New page; prints `Created page with UUID: <uuid>` |
| `cerbo page read <UUID>` | Print the page's markdown |
| `cerbo page write <UUID> <CONTENT>` | Replace the **whole** page content; links and annotations are re-extracted |
| `cerbo page delete <UUID>` | Move the page to `.cerbo/trash/<timestamp>-<uuid>/` (nothing is erased) |
| `cerbo page backlinks <UUID>` | Pages linking to the object, `<uuid>: <title>` per line |
| `cerbo page move <UUID> <PATH>` / `--root` | Set the page's folder in the virtual tree (`notes/rust`), or clear it; exactly one of the two |
| `cerbo folder rename <OLD> <NEW>` | Move every page whose path is `OLD` or under `OLD/` to `NEW`; `0 pages` is not an error |
| `cerbo folder list` | The tree in sidebar order: `name/` folders, `Title  <uuid>` pages, two spaces per level |
| `cerbo folder create <PATH>` / `delete <PATH>` | Pin an empty folder that stays until deleted / delete a folder with no pages |
| `cerbo folder order <FOLDER>` / `--root` `<ENTRY>...` | Set the sidebar order of a folder's children; an entry is a page UUID or a child folder name |
| `cerbo resolve <UUID>` | UUID to filesystem path of the object directory |
| `cerbo info` | Config dir, cache dir, registered vaults |
| `cerbo import <URL>` | A read-only Source object (`schema:WebPage`) for the URL |
| `cerbo import-ontology <URL> [--prefix p --namespace IRI]` | An Ontology object; with both flags, registers prefix `p` for annotations |
| `cerbo index [--vault ID\|PATH] [--page UUID] [--embeddings]` | Rebuild `annotations.ttl` and the semantic index, report broken links; `--embeddings` also computes vectors for search |
| `cerbo sparql '<QUERY>'` / `--file q.rq` / `-` | Read-only SPARQL 1.1 against the vault's graph |
| `cerbo search "<TEXT>"` | Pages closest in meaning to the text, any language |
| `cerbo similar <UUID>` | Pages closest in meaning to a page |
| `cerbo symlink` | Rebuild `<vault>/cerbo/<slug>.md` symlinks into the object store |
| `cerbo migrate vocabulary [--dry-run] [--if-needed]` | Rewrite `meta.ttl` / `annotations.ttl` into the current schema.org vocabulary |

Every command above takes `--json`. `search`, `similar` take `--limit N`,
`--type prefix:Class`, `--annotated prefix:Type` (repeatable).

## Rules for agents

- **Prefer `--json`** when you will parse the output. stdout holds only the
  result; progress and warnings (index build, vocabulary migration, broken
  links) go to stderr.
- **`page write` replaces the whole page.** Read the page first, edit the text,
  write it back. There is no append or patch.
- **Keep links as `cerbo://<uuid>`.** Get a UUID from `page create`, `page list`
  or `cerbo search`; never invent one. A link to a missing object is saved but
  reported as broken on stderr.
- **Deleting is safe but not an undo.** `page delete` moves the directory to
  `.cerbo/trash/`; recover it by moving the directory back to
  `.cerbo/objects/<uuid>/`. Imported Source objects cannot be deleted.
- **Semantic search is opt-in and heavy.** The first `cerbo index --embeddings`
  (or the first `search`/`similar`) downloads a 490 MB model into
  `$XDG_CACHE_HOME/cerbo/models/`. Ask the user before triggering that in a
  fresh environment. Offline machines set `CERBO_MODEL_DIR` to a local copy.
- **The index lives in the user cache**, `$XDG_CACHE_HOME/cerbo/index/`, never
  in the vault. It refreshes itself before each answer, so no `cerbo index` is
  needed after `git pull`. Run a full `cerbo index` after editing `page.md`
  files by hand, bulk imports, or a crash.
- **"index busy"** means another Cerbo process (usually the desktop app) held
  the index for more than 5 seconds. Retry, or close the app.
- **SPARQL is read-only.** `INSERT`/`DELETE`/`LOAD` are refused; change data
  through `page write`.
- **Folders are paths, not objects.** `page move` and `folder rename` only
  rewrite `cerbo:virtualPath` in `meta.ttl`; a folder exists while a page is in
  it, or while it is pinned with `folder create`. Pins and the sidebar order
  live in `.cerbo/tree.toml`. Paths are `a/b/c` with no leading or trailing slash and no `.`/`..`.
  Moving two pages with the same slug into one folder is allowed but makes
  `cerbo symlink` refuse until one slug or path changes.
- **Don't edit `meta.ttl` / `annotations.ttl` by hand.** They are regenerated
  on save and by `cerbo index`.
- A non-zero exit code always means failure; the message is on stderr (or in
  `{"error": true, "message": ...}` with `--json`).

## Workflows

### New vault and first page

```bash
mkdir notes && cd notes
cerbo init
cerbo page create "Trip to Berlin" --json          # {"uuid":"<A>"}
cerbo page create "Alice" --json                   # {"uuid":"<B>"}
cerbo page write <A> "# Trip to Berlin

Went with [Alice](cerbo://<B>) to [Berlin]{schema:Place} in May."
cerbo page backlinks <B>                           # <A>: Trip to Berlin
cerbo page move <A> travel/2026                    # folder in the tree; Moved <A> to travel/2026
cerbo symlink                                      # notes/cerbo/travel/2026/trip-to-berlin.md -> ../../../.cerbo/objects/<A>/page.md
```

### Edit an existing page

```bash
cerbo page read <UUID> > /tmp/page.md     # or: cerbo page read <UUID> --json | jq -r .content
# edit /tmp/page.md
cerbo page write <UUID> "$(cat /tmp/page.md)"
```

### Find things

```bash
cerbo page list --json | jq '.[] | select(.title | test("berlin"; "i"))'
cerbo sparql 'SELECT ?page ?title WHERE { ?page a schema:NoteDigitalDocument ; schema:name ?title } ORDER BY ?title'
cerbo sparql 'SELECT ?page WHERE { ?page schema:mentions ?m . ?m a schema:Place ; schema:name "Berlin" }'
cerbo sparql 'ASK { ?p schema:citation <cerbo://objects/<UUID>> }'
cerbo index --embeddings                   # once per vault
cerbo search "поездка в Берлин" --limit 5 --json
cerbo search "city" --annotated schema:Place --type schema:NoteDigitalDocument
cerbo similar <UUID>
cerbo sparql 'SELECT ?a ?b WHERE { ?a cerbo:similarTo ?b }'
```

Predeclared SPARQL prefixes: `cerbo:`, `schema:`, `foaf:`, `dcterms:`,
`skos:`, `rdf:`, `rdfs:`, `owl:`, `xsd:` and every prefix registered in the
vault's `.cerbo/ontology-map.json`. Links appear as
`<page> schema:citation <target>`; annotations as
`<page> schema:mentions [ a <type> ; schema:name "Text" ]`. Object classes:
`schema:NoteDigitalDocument` (page), `schema:WebPage` (imported source),
`schema:MediaObject` (attachment), `schema:DefinedTermSet` (ontology).

### Import and custom ontologies

```bash
cerbo import https://example.com/article --json                       # {"uuid":..., "url":...}
cerbo import-ontology https://example.org/vocab.ttl --prefix ex --namespace https://example.org/vocab#
cerbo page write <UUID> "[Bob]{ex:Person}"                            # resolves to <https://example.org/vocab#Person>
```

Built-in prefixes (`schema`, `foaf`, `dcterms`, `skos`, `rdf`, `rdfs`, `owl`,
`xsd`) need no registration. An unknown prefix is kept as
`cerbo:unresolvedType` with a warning, not an error.

### Maintenance

```bash
cerbo index --json            # e.g. {"pages_processed":137,"links_found":...,"errors":0,...}
cerbo index --page <UUID>     # just one page, right after your own edit
cerbo index --vault /path     # one vault from outside it
cerbo migrate vocabulary --dry-run
cerbo migrate vocabulary      # refuses to run while the desktop app is open
```

A `cerbo index` run that exits 0 has fully rebuilt the vault's metadata.

## When something is wrong

| Symptom | Cause / fix |
| --- | --- |
| `not inside a cerbo vault` | Run from inside the vault, pass `--vault`, or `cerbo vault active <ID>` |
| `Object not found` | Wrong UUID, or the object is in `.cerbo/trash/` |
| `index busy` | Desktop app or another `cerbo` holds the index; retry |
| `run cerbo index --embeddings` on `search`/`similar` | The vault has no embeddings yet |
| `embedding model unavailable` | No network and no cached model; set `CERBO_MODEL_DIR` |
| Broken link warning on `page write` | The target UUID does not exist; the page was still saved |
| `cerbo symlink` skips pages | They have no `cerbo:slug`; run `cerbo index` first |
| `Cannot specify both --vault and --page` | Pick one for `cerbo index` |

Changing the CLI? Update `cli/man/cerbo.md` in the same change (`cerbo.1` is
generated at build time and not committed).
