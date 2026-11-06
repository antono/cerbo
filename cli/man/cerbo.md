# NAME

cerbo - Semantic knowledge management tool for agents and humans

# SYNOPSIS

`cerbo [OPTIONS] <COMMAND>`

# DESCRIPTION

Cerbo is a local-first markdown wiki tool designed for both human and AI agent workflows. It uses a UUID-based storage model where all content objects are stored as directories named by UUID, making it easy to reference and link between objects.

The tool supports managing multiple vaults (workspaces), creating and editing pages with markdown content, importing web content as source objects, and working with ontologies (Schema.org, FOAF) for semantic markup using the HackMD annotation syntax `[Text]{prefix:Type}`.

# VAULT AUTO-DISCOVERY

All cerbo commands automatically detect the active vault from the current working directory, without requiring a `--vault` flag. At startup, cerbo walks up from the current directory looking for an ancestor directory containing `.cerbo/`. If found, that vault is used for the invocation.

Vault resolution priority (highest to lowest):

1. Explicit `--vault` flag (where available)
2. Vault discovered from current working directory (walk-up)
3. Active vault set via `cerbo vault active <ID>`
4. Single registered vault fallback (when only one vault is registered)
5. Error: not inside a cerbo vault

If the current directory is inside a vault that has not been registered with `cerbo vault add`, it is automatically recorded in `vaults.auto.toml` (silently, no output) and immediately used for the current invocation. Auto-registered vaults appear in `cerbo vault list` with an `(auto)` marker.

# COMMANDS

## init

Initialize a new vault in the current directory. Creates a `.cerbo/` directory with the necessary structure, bundles standard ontologies (Schema.org, FOAF), and adds `/cerbo/` and `/.cerbo/trash/` to `.gitignore`.

Re-running `cerbo init` on an existing vault leaves it alone, but still tops up the `.gitignore` entries without duplicating or reordering anything.

Usage: `cerbo init [OPTIONS]`

Options:
- `--json` - Output result as JSON

## vault

Vault management commands for working with multiple workspaces.

Usage: `cerbo vault <SUBCOMMAND>`

### Subcommands:

- `cerbo vault list [--json]` - List all registered vaults, one per line as `<id>: <name>`. Auto-registered vaults are shown with an `(auto)` marker and the active vault with `(current)`. JSON output includes an `is_auto` boolean field per vault.
- `cerbo vault add <NAME> <PATH> [--json]` - Register the vault at `PATH` (the directory containing `.cerbo/`) under the display name `NAME`, and print its ID
- `cerbo vault remove <ID> [--json]` - Remove a vault from the registry by its ID (works on both manually and auto-registered vaults); the vault's files are left alone
- `cerbo vault active <ID> [--json]` - Set the active vault, used when no vault is found around the current directory (see VAULT AUTO-DISCOVERY)
- `cerbo vault approve <ID> [--json]` - Promote an auto-registered vault to the manual registry. Moves the entry from `vaults.auto.toml` to `vaults.toml`. Errors if the ID is not in the auto registry or the path is already manually registered.

## page

Page management commands for creating, reading, and modifying wiki pages.

Usage: `cerbo page <SUBCOMMAND>`

### Subcommands:

- `cerbo page list [--json] [--vault <ID>]` - List all pages in the current vault (auto-discovered from CWD, or override with --vault): a header line naming the vault, then one `<uuid>: <title>` per page, followed by ` [<virtual path>]` for a page inside a folder. With `--json`, a JSON array of objects with `uuid`, `title` and `virtualPath` (a string, or `null` at the root).
- `cerbo page create <TITLE> [--json]` - Create a new page with the given title and print its UUID (`Created page with UUID: <uuid>`; with `--json`, `{"uuid": ...}`)
- `cerbo page read <UUID> [--json]` - Print the markdown content of a page (with `--json`, `{"content": ...}`)
- `cerbo page write <UUID> <CONTENT> [--json]` - Replace the whole content of a page with `CONTENT`; its links and annotations are re-extracted, and a link to a missing object is reported on stderr while the save still succeeds
- `cerbo page delete <UUID> [--json]` - Delete a page by its UUID (moved to the trash, see below)
- `cerbo page move <UUID> <PATH> [--json]` / `cerbo page move <UUID> --root [--json]` - Set the page's virtual path (its folder in the symlink tree and in the desktop sidebar) to `PATH`, or clear it with `--root`. Exactly one of `PATH` and `--root` is required. `PATH` is segments joined by `/` with no leading or trailing slash, no empty segment and no `.` or `..` segment; an invalid path exits non-zero with the broken rule on stderr and writes nothing. Only the page's own `meta.ttl` changes; slug, title and content stay. Imported Source pages can be moved. Prints `Moved <uuid> to <path>` (or `to root`); with `--json`, `{"uuid": ..., "virtualPath": <string or null>}`.
- `cerbo page backlinks <UUID> [--json] [--vault <ID>]` - List the pages that link to an object, one per line as `<uuid>: <title>`, ordered by title then UUID. With `--json`, stdout is only a JSON array of objects with `uuid` and `title`. An unknown UUID exits non-zero with "Object not found".

**Backlinks come from the semantic index.** A page's links are stored only in its own `page.md`; saving a page never writes to the objects it links to. Cerbo keeps a per-machine index of every page's links in the user cache (`$XDG_CACHE_HOME/cerbo/index/`, never inside the vault; see SEMANTIC INDEX) and answers `cerbo page backlinks` from it. Before answering, the index picks up pages that were added, edited or deleted outside Cerbo (by git, file sync or an editor), so a synced page that links here is listed without running `cerbo index`. The first command that needs the index builds it, with progress on stderr. While another Cerbo process (such as the desktop app) is using the index, the command waits up to 5 seconds and then fails with "index busy". Deleting the index directory loses nothing; it is rebuilt from the vault files.

**Delete moves the page to the trash.** `cerbo page delete` renames the object's directory to `.cerbo/trash/<timestamp>-<uuid>/` rather than removing it. Every file inside is preserved byte-for-byte, so a mistaken delete is recovered by moving the directory back to `.cerbo/objects/<uuid>/`. Deleting, recreating and deleting the same UUID again leaves two separate entries; neither overwrites the other.

Trashed objects are outside the live set: they do not appear in `cerbo page list`, do not resolve, are not indexed, and get no symlink. A page that still links to a trashed object reports a broken link — the object is never resurrected.

Nothing prunes the trash. Remove entries yourself when you are sure, for example with `rm -rf .cerbo/trash/<entry>`.

Source-type (imported) objects are read-only and cannot be deleted; the refusal happens before anything is moved.

## folder

Operate on the folders of the virtual tree. A folder exists while at least one page's `cerbo:virtualPath` is that path or lies under it, or while it is pinned (created with `folder create`); an unpinned folder disappears when the last page moves out.

The vault's sidebar layout lives in `.cerbo/tree.toml`: `pinned`, the folders that exist with no page in them, and `order`, a table mapping a folder path (`""` is the root) to its children in display order, where a page is its UUID and a child folder is its name followed by `/`. Children named there come first, in that order; the others follow with folders before pages, each by name or title, case-insensitively. Entries that name nothing are skipped. The file is optional; one that does not parse is reported and never overwritten. The desktop app reads the same file.

Usage: `cerbo folder <SUBCOMMAND>`

### Subcommands:

- `cerbo folder list [--json]` - Print the tree in its stored order, two spaces of indent per level: a folder as `name/`, a page as `Title  <uuid>` (two spaces before the UUID). With `--json`, nested nodes `{"kind": "folder", "name", "path", "pinned", "children"}` and `{"kind": "page", "uuid", "title"}`.
- `cerbo folder create <PATH> [--json]` - Create an empty folder by pinning `PATH` in `.cerbo/tree.toml`; it stays in the tree until deleted, with or without pages. A folder that already exists (pinned, or from page paths) is refused with `folder already exists: <path>`. Prints `Created folder <path>`; with `--json`, `{"path": ...}`.
- `cerbo folder delete <PATH> [--json]` - Delete an empty folder: unpin it and everything under it and drop its order lists and its place in the parent's order. A folder with pages under it is refused with `folder is not empty: <path> (<n> pages)`; move or delete the pages first. Prints `Deleted folder <path>`; with `--json`, `{"path": ...}`.
- `cerbo folder order <FOLDER> <ENTRY>... [--json]` / `cerbo folder order --root <ENTRY>... [--json]` - Set the display order of a folder's children (or the root's). An `ENTRY` is a page UUID or a child folder name, with or without a trailing `/`. Children left out follow in the default order; entries naming no child of that folder are dropped, and a malformed entry fails before anything is written. Prints `Ordered <folder|root> (<n> entries)`; with `--json`, `{"folder": <path, "" for the root>, "entries": [...]}` as stored.
- `cerbo folder rename <OLD> <NEW> [--json]` - Replace the folder prefix `OLD` with `NEW` on every page whose virtual path is `OLD` or starts with `OLD/`; nested folders move along (`notes/rust` becomes `archive/rust` when `notes` is renamed to `archive`), and `notebook` is not under `notes`. Both paths must be non-empty and valid; that is checked before any page is written. Renaming onto an existing folder merges the two. Each page's `meta.ttl` is written on its own; a page that cannot be written is reported on stderr as `Error: could not move <uuid>: <reason>`, the others are still moved, and the command exits non-zero. Prints `Renamed <old> to <new> (<n> pages)`; with `--json`, `{"from": ..., "to": ..., "pages": <n>, "failed": [<uuid>, ...]}`. With no page under `OLD` it exits zero with `0 pages`. The layout follows: pinned folders and order lists at or under `OLD` move to `NEW`, the folder keeps its place in its parent's order when the parent stays the same, and a pinned folder with no pages is renamed too.

A slug collision inside the target folder (two pages that would share `<folder>/<slug>.md`) is not refused by `page move` or `folder rename`; `cerbo index` reports it and `cerbo symlink` refuses to build until it is resolved.

## resolve

Resolve a UUID to its local filesystem path.

Usage: `cerbo resolve <UUID> [--json]`

Arguments:
- `UUID` - The UUID of the object to resolve

Options:
- `--json` - Output result as JSON with path field

## info

Show configuration and vault information.

Usage: `cerbo info [--json]`

Options:
- `--json` - Output result as JSON with config_dir, cache_dir, and vaults

## import

Import a URL as a Source object (read-only reference to external content).

Usage: `cerbo import <URL> [--json]`

Arguments:
- `URL` - The URL to import

Options:
- `--json` - Output result as JSON with uuid and url fields

## import-ontology

Import an ontology file and create an Ontology object (`rdf:type schema:DefinedTermSet`).

With `--prefix` and `--namespace`, the vault's `.cerbo/ontology-map.json` then maps the prefix to the namespace IRI and the new object's UUID, so `[Text]{prefix:Type}` annotations resolve to `<namespace>Type`. The two flags must be given together; with only one of them the command fails and creates nothing. Without them the object is created, the map is not changed, and a warning says that annotations cannot use the ontology until it is registered.

The built-in prefixes (`schema`, `foaf`, `dcterms`, `skos`, `rdf`, `rdfs`, `owl`, `xsd`) resolve without any registration; register one only to bind it to a different namespace.

Usage: `cerbo import-ontology <URL> [--prefix <PREFIX> --namespace <IRI>] [--json]`

Arguments:
- `URL` - The URL of the ontology file (e.g., Schema.org or FOAF)

Options:
- `--prefix <PREFIX>` - Annotation prefix to register (requires `--namespace`)
- `--namespace <IRI>` - Namespace IRI the prefix expands to, e.g. `https://example.org/vocab#` (requires `--prefix`)
- `--json` - Output result as JSON with uuid, url, prefix and namespace fields

## import-vault

Import an Obsidian or Tolaria vault (a folder of Markdown notes with YAML frontmatter and `[[wikilinks]]`) into a Cerbo vault. Every note becomes a page, links become `cerbo://` links, referenced files become Attachment objects, and frontmatter becomes RDF in `meta.ttl`. The source folder is only read: no file under it is created, changed, moved or deleted.

Usage: `cerbo import-vault <DIR> [--from obsidian|tolaria] [--vault <ID> | --new <PATH> [--name <NAME>]] [--force] [--dry-run] [--json]`

Arguments:
- `DIR` - The folder to import (the Obsidian or Tolaria vault)

Options:
- `--from <FORMAT>` - Source format, `obsidian` or `tolaria`; overrides detection
- `--vault <ID>` - Import into this registered vault (an ID from `cerbo vault list`). An unknown ID fails with "Vault not found" and writes nothing.
- `--new <PATH>` - Create a vault in `PATH` (missing, or empty but for `.git`), initialise it as `cerbo init` does, register it, and import into it. A folder holding other files is refused. The vault is created only after the source has been read and planned, so a source that cannot be imported leaves no empty vault behind.
- `--name <NAME>` - Name of the vault created by `--new` (default: the source folder name). Needs `--new`.
- `--force` - Import a folder that was already imported into this vault: makes a second, separate copy with its own ontology and prefix
- `--dry-run` - Read the source and print the report without asking for a vault or writing anything (no vault, no registry file)
- `--json` - Print only the report, as one JSON object (see below). An error prints `{"error": true, "message": ...}`.

`--vault` and `--new` cannot be combined: that is a usage error (exit status 2).

**Target vault:** without `--vault` or `--new`, on a terminal, the command lists the registered vaults by number (name and path) plus "Create a new vault" on stderr and reads the answer from stdin; "Create a new vault" then asks for a folder (`~/` is expanded) and a name. When stdin is not a terminal the command fails with "no target vault: pass --vault <ID> or --new <PATH>" and writes nothing.

**Format detection:** a folder with `.obsidian/` is Obsidian. A folder without it that has a `views/` folder of `.yml` files, or a Markdown note with a `type:` frontmatter field, is Tolaria. Anything else fails with "cannot detect vault format, pass --from obsidian|tolaria". A folder holding `.cerbo/` is a Cerbo vault and is refused, also with `--from`. The report says which format was used and why.

**Mapping:**
- Note (`*.md`) - a page (`rdf:type schema:NoteDigitalDocument`); `page.md` holds the body without its frontmatter
- Title - Tolaria: the first H1, else the file stem; Obsidian: the file stem. `# <title>` is prepended when the body does not start with it.
- Folder - `cerbo:virtualPath` (the page's folder in the sidebar and symlink tree)
- File stem - `cerbo:slug`, transliterated to ASCII; a second note in one folder with the same slug gets `-2`, then `-3`, and is listed under renamed slugs
- File times - `schema:dateCreated` (birth time, else modification time) and `schema:dateModified`
- `[[Note]]`, `[[Note|text]]`, `[[Note#Heading]]`, `![[Note]]` - `[text](cerbo://<uuid>)`, where text is the `|text` part, else the target's title; the `#Heading` is dropped and counted. A target matches a note by relative path, then file stem, then title, then Obsidian alias, ignoring case; with several matches the one nearest the linking note's folder wins and the link is listed as ambiguous.
- `[text](other.md)`, `[text](../other%20note.md)` - `[text](cerbo://<uuid>)`; external URLs are left alone
- `![[photo.png]]`, `![alt](path)`, `[text](path)` to a local file - one Attachment object per file, however many notes use it, linked as `![alt](cerbo://<uuid>)` or `[text](cerbo://<uuid>)`. Files are matched by relative path, then by file name anywhere in the vault. Files no note references are listed, not imported.
- Links inside code blocks and inline code - left unchanged
- Unresolved links - kept as written and listed in the report
- Frontmatter `type: X` (Tolaria) - a second `rdf:type <ns>X`; a note with `type: Type` declares the class `X` in the import's ontology, labelled with its title
- Frontmatter values holding `[[wikilinks]]` - one `<ns><key> <cerbo://objects/<uuid>>` relation per resolved link; an unresolved one is kept as a string and listed
- Other frontmatter values - literals `<ns><key>`: strings, booleans, integers, ISO dates and date-times with their XSD type, one triple per list item, nested maps as a YAML string. Keys are percent-encoded where they are not IRI-safe (`Belongs to` - `Belongs%20to`).
- Keys starting with `_` (Tolaria app state such as `_favorite`) - dropped
- Obsidian `tags` / `aliases` - `schema:keywords` (without `#`) / `schema:alternateName`; aliases also resolve wikilinks. Inline `#tags` stay text.
- Frontmatter that cannot be parsed - kept at the top of `page.md` in a `yaml` code block and listed in the report
- Skipped and listed: Obsidian `.canvas` and `.excalidraw.md` files, Tolaria `views/`, symbolic links. Left out silently: `.obsidian/`, `.git/`, `.trash/` and every other hidden file or folder.

`<ns>` is the import's own namespace `cerbo://vocab/<uuid>/`, owned by one Ontology object titled `<Format>: <source folder name>`. That object records the source as a `file://` URL (`schema:url`), the format (`cerbo:importFormat`) and the import time, and its `meta.ttl` declares every class and property the import used. Its prefix, the source folder name slugified (`braindump`, or `braindump-2` when taken), is registered in `.cerbo/ontology-map.json`, so annotations can write `[Text]{braindump:Journal}`.

**One import per source:** importing a folder into a vault that already holds an import of the same folder (same canonical path, in any format) fails before anything is written, naming the earlier import's date and format. `--force` imports a second copy. Updating an earlier import is not supported.

**Target outside the source:** the target vault (from `--vault`, `--new` or the prompt) may not be the source folder or lie inside it, since the source is never written; such a target is refused before anything is created. Only one import runs into a vault at a time; a second one started meanwhile is refused.

**All or nothing:** objects are written to `.cerbo/import-staging/` first and then moved into `.cerbo/objects/`; the prefix map is updated last. A failed import leaves the vault's pages and prefix map as they were; should some moved objects not go back to the trash, the next import into that vault finishes the cleanup. If an import was interrupted while moving, the next `cerbo import-vault` into that vault moves its objects to the trash first and reports how many. After the import the semantic index is brought up to date, so `cerbo page backlinks` and `cerbo sparql` see the new pages without `cerbo index`; should that fail, the import still succeeds and the report carries a warning.

**Report:** the human report says `Imported N pages and M attachments (size) into "<vault>" (<id>)` (`Dry run: would import ...` with `--dry-run`), the source and format, the ontology and prefix, the counts of rewritten links, relations and dropped heading fragments, then the lists that need attention: skipped files, unreferenced files, unresolved links (with the note and, for frontmatter, the key), ambiguous links (with the chosen target and the candidates), renamed slugs and frontmatter errors. Warnings go to stderr. With `--json` the report is one JSON object with the fields `vault` (`{id, name, path}`; `null` on a dry run), `source`, `format`, `formatReason`, `dryRun`, `pages`, `attachments`, `attachmentBytes`, `relations`, `links`, `droppedFragments`, `ontology` (`{uuid, title, namespace, prefix}`), `skipped`, `unreferenced`, `unresolved` (`[{note, link, property}]`), `ambiguous` (`[{note, link, chosen, candidates}]`), `renamedSlugs` (`[{note, from, to}]`), `frontmatterErrors` (`[{note, error}]`), `recovered` and `warnings`.

Examples:

```
# See what an Obsidian vault would become, without writing anything
cerbo import-vault ~/Notes --dry-run

# Import it into a registered vault
cerbo import-vault ~/Notes --vault 3f2a

# Import a Tolaria vault into a new vault named Braindump
cerbo import-vault ~/Sync/Tolaria/Braindump --from tolaria --new ~/cerbo/braindump --name Braindump --json
```

## index

Rebuild metadata (annotations.ttl) from page content, report broken links, and bring the semantic index up to date. Used for recovery after manual edits or bulk imports. Backlinks are not files; see `cerbo page backlinks` and SEMANTIC INDEX.

Usage: `cerbo index [OPTIONS]`

Without options the command indexes every registered vault (see `cerbo vault list`), plus the vault around the current directory should it not be registered; a registered vault whose directory is missing is skipped with a warning. With more than one vault, each vault's name is printed on stderr before its warnings, and the summary adds up all of them. A vault that cannot be indexed (for example because another Cerbo process holds its index) does not stop the others: it is named on stderr after the summary and the run exits non-zero.

Options:
- `--vault <ID|PATH>` - Index only this vault: a vault ID from `cerbo vault list`, or a vault path
- `--page <UUID>` - Index only a single page (incremental mode), in the vault around the current directory if the page is there, otherwise in the registered vault that holds it. Cannot be combined with `--vault`: `cerbo index --vault <id> --page <uuid>` exits non-zero with "Cannot specify both --vault and --page".
- `--no-backfill-slug` - Skip backfilling missing `cerbo:slug` values into `meta.ttl`
- `--embeddings` - Also compute the embeddings of every page whose content changed since its last embedding and refresh the similarity links (see SEMANTIC SEARCH). The first run on a vault creates its embeddings and may download the model.
- `--json` - Output result as JSON with pages_processed, links_found, annotations_found, triples, slugs_backfilled, path_errors, collisions, errors, vault_errors (the number of vaults that could not be indexed), and with `--embeddings` also embedded

**Behavior:**
- Without `--page`: Full rebuild of each vault in scope. Every page's `annotations.ttl` is regenerated and its links are counted; a link to an object that does not exist is reported as broken on stderr. No file of a link target is written. The semantic index is then rebuilt from the vault files, so it matches them afterwards; the previous index keeps answering until the new one is complete.
- With `--page <UUID>`: Updates only the annotations of that one page, checks its links, and replaces only that page's triples in the semantic index.
- With `--embeddings`: after the above, every page of the vault (also with `--page`) whose title or `page.md` changed since its last embedding is embedded, vectors of deleted pages are dropped, and the `cerbo:similarTo` links are recomputed. Unchanged pages are not embedded again. Progress goes to stderr.
- The summary reports the number of triples in the semantic index; with `--embeddings` it also reports the number of pages embedded. With `--json` these are the numeric fields `triples` and `embedded` (`embedded` only with `--embeddings`).
- If the semantic index has to be built first, its progress goes to stderr.
- Idempotent: Running twice produces identical files and an index that answers every query identically
- Continues past an object it cannot read or write, then reports every one of them by UUID on stderr. An object whose `meta.ttl` names more than one object class is reported and left untouched: it is neither indexed nor given a slug.

**Exit status:** `cerbo index` exits non-zero if any object could not be read or written, naming each on stderr, if the semantic index could not be updated (for example because another Cerbo process held it for more than 5 seconds), if any vault in scope could not be indexed, or, with `--embeddings`, if the embedding model is unavailable (see SEMANTIC SEARCH). The vault files are written either way. A run that exits zero has fully rebuilt the vault's metadata.

**When to use `--page`:**
- Immediately after your own edit to one page, to refresh just what that page declares

`--page` cannot repair a vault; it only looks at the named page. After bulk imports or a crash, run a full-vault reindex (`cerbo index` with no `--page`).

## sparql

Run a read-only SPARQL 1.1 query against the vault's semantic index (see SEMANTIC INDEX). Before answering, the index picks up pages added, edited or deleted outside Cerbo; the first query builds it, with progress on stderr.

Usage: `cerbo sparql [OPTIONS] <QUERY|->`

Arguments:
- `QUERY` - The query text. `-` reads the query from stdin.

Options:
- `--file <PATH>` - Read the query from a file instead (cannot be combined with `QUERY`)
- `--vault <PATH>` - Explicit vault path (default: discover from the current directory like Git)
- `--json` - Print SELECT and ASK results as SPARQL 1.1 Query Results JSON, and CONSTRUCT and DESCRIBE results as JSON-LD. Nothing else is printed to stdout.

**Output without `--json`:**
- SELECT: a table with one column per variable, a header line and a rule. IRIs are shown bare and literals by their value; an unbound variable is an empty cell. Use `--json` for exact terms.
- ASK: `true` or `false`
- CONSTRUCT and DESCRIBE: a Turtle document

**Predeclared prefixes:** a query may use these without a `PREFIX` line: `cerbo:` (`cerbo://ontology/`), the built-in prefixes `schema:` (`https://schema.org/`), `foaf:`, `dcterms:`, `skos:`, `rdf:`, `rdfs:`, `owl:` and `xsd:`, and every prefix in the vault's `.cerbo/ontology-map.json`. A vault map entry overrides a built-in prefix, and a `PREFIX` in the query overrides both.

**What the graph holds:** each object's `meta.ttl` and `annotations.ttl` triples, plus `<page> schema:citation <target>` for every distinct `cerbo://` link in its `page.md` (links to missing objects included). Each object's triples are in the named graph `cerbo://graphs/<uuid>`. Once the vault has embeddings, the graph `cerbo://graphs/similarity` also holds `<page> cerbo:similarTo <other>` for each page's closest pages (see SEMANTIC SEARCH). Without `FROM` or `FROM NAMED`, the default graph is the union of all graphs, so plain patterns see every object.

**Read-only:** SPARQL UPDATE (`INSERT`, `DELETE`, `LOAD`, ...) is refused with a non-zero exit: vault data is changed through files, not the index.

**Exit status:** non-zero when the query does not parse (stderr gives the parser's message with the `line:column` position of the error), when it fails while running (for example `SERVICE`, which is not supported), for an update, or when the index is busy for more than 5 seconds.

## search

Find the pages closest in meaning to a text, best first. The text and the pages may be in different languages (for example a Russian query finds an English page). Needs embeddings: run `cerbo index --embeddings` once per vault (see SEMANTIC SEARCH).

Usage: `cerbo search [OPTIONS] <TEXT>`

Options:
- `--limit <N>` - Return at most N pages (default 10, at least 1)
- `--type <CLASS>` - Keep only objects with this `rdf:type`, for example `schema:WebPage`. Repeat to require several.
- `--annotated <TYPE>` - Keep only pages with an annotation of this type, for example `schema:Place`. Repeat to require several.
- `--vault <PATH>` - Explicit vault path (default: discover from the current directory like Git)
- `--json` - Print only a JSON array of objects with the fields `uuid`, `title` and `score`

`CLASS` and `TYPE` are `prefix:Name` (with the same predeclared prefixes as `cerbo sparql`), `<IRI>` or an absolute IRI. Filters apply before `--limit`: Cerbo keeps looking further down the ranking until N pages pass or the vault has no more.

**Output without `--json`:** one line per page, `<score>  <uuid>: <title>`, where the score is the cosine similarity (higher is closer).

**Fresh results:** before searching, pages added, edited or deleted since their last embedding are embedded again, so a page saved a moment ago is found. Saving a page never computes an embedding itself.

**Exit status:** non-zero when the vault has no embeddings yet (the message says to run `cerbo index --embeddings`), when a filter names an unknown prefix, when the embedding model is unavailable, or when the index is busy for more than 5 seconds.

## similar

Find the pages closest in meaning to a page, best first, without the page itself. Needs embeddings, like `cerbo search`.

Usage: `cerbo similar [OPTIONS] <UUID>`

Options: the same as `cerbo search` (`--limit`, `--type`, `--annotated`, `--vault`, `--json`), with the same output.

**Exit status:** non-zero with "Object not found" when `UUID` is not an object of the vault, and in the cases listed for `cerbo search`. An object without a page (for example an attachment) has no similar pages.

## migrate vocabulary

Rewrite every object's `meta.ttl` and `annotations.ttl` into the current schema.org vocabulary, exactly as a save writes them (see STORAGE). A vault written by the current Cerbo is left unchanged.

Usage: `cerbo migrate vocabulary [OPTIONS]`

Options:
- `--vault <ID|PATH>` - Migrate only this vault, given by registered vault ID or by path (default: every registered vault)
- `--dry-run` - List the files that would be rewritten and write nothing
- `--if-needed` - Check cheaply, using cached file stamps in `$XDG_CACHE_HOME/cerbo/vocabulary/`, and rewrite only the objects that need it. Prints nothing when every vault is current. Cannot be combined with `--dry-run`
- `--json` - Print only a JSON object with the numeric fields scanned, rewritten and errors

**Behavior:**
- `meta.ttl` is rewritten with `rdf:type`, `schema:name`, `schema:url` and `schema:encodingFormat`; values (title, dates, slug, virtual path) and any other triples are kept.
- `annotations.ttl` is regenerated from `page.md` with the vault's prefix map when it needs rewriting or does not parse; a page with no `annotations.ttl` does not get one. Ontology objects are skipped, as in `cerbo index`.
- Each file is written atomically, and only when its new content differs from what is on disk. A second run rewrites nothing and changes no modification time.
- Objects in `.cerbo/trash/` are not touched.
- An object that cannot be read (for example a `meta.ttl` that does not parse, or one with two object classes) is named by UUID on stderr and left untouched; the other objects are still migrated. So is an object one of whose files changed while it was being migrated.
- Without `--dry-run`, it refuses to run while a Cerbo desktop app (or another migration) is running, since the app may be saving the same files: it prints an error and writes nothing. Close the app and run it again.
- Unlike other commands, `cerbo migrate` never auto-registers the vault in the current directory; use `--vault` to name an unregistered one.

**Exit status:** non-zero if any object or vault could not be migrated, or the desktop app is running.

## symlink

Build (or rebuild) a human-readable symlink tree at `<repo-root>/cerbo/`. Each page (Product or Source object) with a `cerbo:slug` value is exposed as a relative symlink named `<slug>.md` pointing directly to the page file inside the object store. Pages that declare a `cerbo:virtualPath` are nested inside corresponding subdirectories. The tree is rebuilt atomically: a new staging directory is written and then swapped into place so readers always see a consistent state.

Usage: `cerbo symlink [OPTIONS]`

The command discovers the vault using Git-style directory traversal: it walks up from the current directory looking for a `.cerbo/` directory. You can also specify an explicit vault path with `--vault`.

Options:
- `--vault <PATH>` - Explicit vault path (overrides Git-style discovery)
- `--json` - Output result as JSON with objects_scanned, leaves_created, dirs_created

**Behavior:**
- Symlinks are always relative (portable when the vault is moved or mounted elsewhere)
- Only Product and Source objects are linked; Attachment and Ontology objects are skipped
- Objects missing `page.md` are skipped with a warning on stderr
- Pages without a `cerbo:slug` are skipped (run `cerbo index` first to backfill slugs)
- Detects leaf-vs-leaf and dir-vs-leaf collisions and aborts with an error listing all conflicts
- Refuses to wipe `cerbo/` if it contains entries that were not created by cerbo (safe-wipe guard)
- `cerbo init` adds `/cerbo/` to `.gitignore` automatically

**When to use:**
- After `cerbo init` to create the initial symlink tree
- After creating or renaming pages (to refresh the tree)
- After moving the vault to a new location (symlinks are relative, so re-running fixes any stale links)

# SEMANTIC INDEX

Cerbo keeps a derived, per-machine index of each vault's knowledge graph: every object's `meta.ttl` and `annotations.ttl` and the links in its `page.md`. `cerbo page backlinks`, `cerbo sparql`, `cerbo search`, `cerbo similar` and the desktop backlinks panel are answered from it.

- **Location:** `$XDG_CACHE_HOME/cerbo/index/<key>/` (default `~/.cache/cerbo/index/`), where `<key>` is derived from the vault's absolute path. Nothing is written into the vault, so the index is never synced or committed.
- **Built automatically:** the first command that needs it builds it, and it is rebuilt when it is from another Cerbo version or the vault moved. Before every answer it picks up files changed outside Cerbo (git, file sync, an editor). `cerbo index` rebuilds it on request.
- **Safe to delete:** removing the index directory loses nothing; it is rebuilt from the vault files. An interrupted rebuild leaves the previous index working.
- **One process at a time:** an operation holds the index briefly. Another Cerbo process waits up to 5 seconds, then reports "index busy" and exits non-zero. Saving a page never fails because of the index: if it cannot be updated, a warning says so, the index is marked stale, and the next operation rebuilds it. The CLI waits up to 5 seconds before marking it; the desktop app never waits, and its own index worker catches up from the file watcher.
- **Orphans:** indexes of vaults that were moved or deleted stay in the cache until you remove them by hand.

# SEMANTIC SEARCH

`cerbo search`, `cerbo similar` and `cerbo index --embeddings` compare pages by meaning with text embeddings: each page (its title and `page.md`) becomes a vector of numbers, and pages with close vectors are about the same thing.

- **Local only:** embeddings are computed on this machine. Page text is never sent anywhere; the only network access is the one-time model download.
- **Opt-in per vault:** a vault has no embeddings until `cerbo index --embeddings` runs on it. Until then `search` and `similar` fail and say so. Embeddings are stored with the semantic index in the user cache, never in the vault, and survive `cerbo index`.
- **Pinned model:** Cerbo uses `intfloat/multilingual-e5-small` (384 dimensions, 512-token window; English, Chinese, Hindi, Spanish, French and about 100 other languages) at one fixed revision, with the SHA-256 checksum of every file built in. Every machine running the same Cerbo release uses byte-identical model files.
- **Download on first use:** the first command that needs the model downloads its files (about 490 MB) from Hugging Face into `$XDG_CACHE_HOME/cerbo/models/<revision>/`, with progress on stderr, and later commands reuse them. Each file is checked before use. A downloaded or cached file whose checksum differs is deleted and the command fails naming the file; run the command again to download it anew.
- **`CERBO_MODEL_DIR`:** when set, the model files are read from that directory instead (laid out as in the model repository: `onnx/model.onnx`, `onnx/tokenizer.json`, `onnx/config.json`, `onnx/special_tokens_map.json` and `onnx/tokenizer_config.json`). They are checked against the pinned checksums, nothing is downloaded, and a mismatching file fails the command with its name (it is not deleted). Use this for machines without network access.
- **Offline:** once the model is cached (or `CERBO_MODEL_DIR` is set), all three commands work without a network. If the model is neither cached nor downloadable, the command fails with "embedding model unavailable".
- **ONNX Runtime:** the model runs through ONNX Runtime 1.24 or newer, loaded from `ORT_DYLIB_PATH`; the packaged `cerbo` sets it.
- **When the model runs:** saving a page never runs it. `search` and `similar` first embed the pages whose content changed since their last embedding; a command that needs no new vector (for example `similar` when nothing changed) does not load the model.
- **Similarity links:** after embeddings are refreshed, each page is linked to up to 5 pages whose similarity is at least 0.80 with `cerbo:similarTo`, in the index graph `cerbo://graphs/similarity`. Each link also has a `cerbo:Similarity` node with `cerbo:source`, `cerbo:target` and `cerbo:score` (an `xsd:decimal`). These links exist only in the index; no vault file contains them. Query them with `cerbo sparql`.

# OPTIONS

Global options available for all commands:

- `-h, --help` - Print help information
- `-V, --version` - Print version information

Command-specific options are listed under each command in the COMMANDS section.

JSON output mode (`--json`): Most commands support a `--json` flag that outputs results in JSON format instead of human-readable text. This is useful for scripting and integration with other tools.

# STORAGE

Cerbo uses a **UUID-based storage model** where all content is stored locally in a `.cerbo/` directory.

## Directory Structure

```
<vault-path>/
  .cerbo/
    +-- objects/
    |   +-- <uuid>/
    |       +-- page.md        # Markdown content (for pages)
    |       +-- meta.ttl       # RDF metadata (Turtle format)
    |       +-- annotations.ttl # HackMD-style annotations
    +-- trash/
    |   +-- <timestamp>-<uuid>/ # Deleted objects, kept until you remove them
    +-- vaults.toml           # Vault registry (manually registered)
    +-- vaults.auto.toml      # Auto-registered vaults (CWD discovery)
    +-- ontology-map.json      # Annotation prefix -> namespace IRI map
    +-- ui.toml               # UI settings
    +-- state.toml            # Application state
```

Derived data is kept outside the vault, in the user cache: the semantic index (with the vault's embeddings) lives in `$XDG_CACHE_HOME/cerbo/index/` (see SEMANTIC INDEX), the embedding model in `$XDG_CACHE_HOME/cerbo/models/` (see SEMANTIC SEARCH), the file stamps `cerbo migrate vocabulary --if-needed` uses in `$XDG_CACHE_HOME/cerbo/vocabulary/`, and the lock a running desktop app holds in `$XDG_CACHE_HOME/cerbo/desktop.lock`.

## Object Types

Each object's `meta.ttl` records its type as `rdf:type` with a schema.org class (`schema:` is `https://schema.org/`):

- **Product** (`schema:NoteDigitalDocument`) - Default type for pages created by users
- **Source** (`schema:WebPage`) - Imported URLs (read-only references); the address is in `schema:url`
- **Attachment** (`schema:MediaObject`) - File attachments (future); the MIME type is in `schema:encodingFormat`
- **Ontology** (`schema:DefinedTermSet`) - Ontology definitions (Schema.org, FOAF bundled by default)

The title is `schema:name`; dates are `schema:dateCreated` and `schema:dateModified`. Further `rdf:type` values are ignored. An object carrying two or more of the four classes above is refused with an error naming its UUID.

## Links and Annotations

Links between objects use the format: `cerbo://<uuid>` (or `cerbo://objects/<uuid>`). A link lives only in the linking page's `page.md`; there is no backlink file. Backlinks are derived from the links in the semantic index (`cerbo page backlinks`). A link to an object that does not exist is reported as broken when the page is saved, and the save still succeeds; no object is created for it.

Annotations use HackMD syntax: `[Text]{prefix:Type}`. The prefix is resolved with the vault's own `.cerbo/ontology-map.json`, which maps each prefix to a namespace IRI (and optionally the ontology object documenting it); the type becomes the namespace IRI followed by `Type`. The prefixes `schema`, `foaf`, `dcterms`, `skos`, `rdf`, `rdfs`, `owl` and `xsd` are built in and need no entry; a map entry for one of them takes precedence. An annotation with an unknown prefix is kept with `cerbo:unresolvedType "prefix:Type"` and a warning.

`annotations.ttl` is rewritten whenever the page is saved (`cerbo page write` or the desktop editor) and by `cerbo index`; a page saved without annotations keeps no `schema:mentions`. Each annotation is stored in `annotations.ttl` as an entity the page mentions: `<page> schema:mentions [ a <type IRI> ; schema:name "Text" ; cerbo:position "line:column" ]`.

Example: `[Alice]{foaf:Person}` - the page mentions a `http://xmlns.com/foaf/0.1/Person` named "Alice"

# EXAMPLES

List the pages linking to a page, as JSON:
```
cerbo page backlinks <UUID> --json
```

Initialize a new vault in the current directory:
```
cerbo init
```

List all registered vaults:
```
cerbo vault list
```

Promote an auto-registered vault to the manual registry:
```
cerbo vault approve <ID>
```

Create a new page:
```
cerbo page create "My First Page"
```

Read a page by UUID (output as JSON):
```
cerbo page read 550e8400-e29b-41d4-a716-446655440000 --json
```

Import a webpage as a source:
```
cerbo import https://example.com/article
```

Import an ontology and register its annotation prefix:
```
cerbo import-ontology https://example.org/vocab.ttl --prefix ex --namespace https://example.org/vocab#
```

Preview, then import an Obsidian vault into a registered vault:
```
cerbo import-vault ~/Notes --dry-run
cerbo import-vault ~/Notes --vault 3f2a
```

Import a Tolaria vault into a new vault:
```
cerbo import-vault ~/Sync/Tolaria/Braindump --new ~/cerbo/braindump --name Braindump
```

Preview, then run, the vocabulary migration of one vault:
```
cerbo migrate vocabulary --vault /path/to/my/vault --dry-run
cerbo migrate vocabulary --vault /path/to/my/vault
```

Show configuration info:
```
cerbo info
```

Rebuild metadata and the semantic index for all pages in current vault:
```
cerbo index
```

Rebuild metadata for a single page (incremental):
```
cerbo index --page 550e8400-e29b-41d4-a716-446655440000
```

Index a vault from outside its directory:
```
cerbo index --vault /path/to/my/vault
```

Get indexing statistics as JSON:
```
cerbo index --json
```

List every page title with SPARQL:
```
cerbo sparql 'SELECT ?title WHERE { ?page schema:name ?title } ORDER BY ?title'
```

Ask whether any page mentions a place, from a file or stdin, as JSON:
```
cerbo sparql --file places.rq
echo 'ASK { ?page schema:mentions ?x . ?x a schema:Place }' | cerbo sparql --json -
```

Embed the vault's pages (downloads the model on first use), then search by meaning:
```
cerbo index --embeddings
cerbo search "поездка в Берлин"
cerbo search "city" --annotated schema:Place --limit 5 --json
```

Find pages similar to a page, using a model copied to this machine and no network:
```
CERBO_MODEL_DIR=/srv/models/multilingual-e5-small cerbo similar 550e8400-e29b-41d4-a716-446655440000
```

List similar pages with SPARQL:
```
cerbo sparql 'SELECT ?a ?b WHERE { ?a cerbo:similarTo ?b }'
```

Build symlink tree:
```
cerbo symlink
```

## Organise pages into folders

```
cerbo page move <uuid> notes/rust          # the page now lives in notes/rust
cerbo page list                            # <uuid>: Ownership [notes/rust]
cerbo folder rename notes archive          # every page under notes/ moves under archive/
cerbo page move <uuid> --root              # back to the root
cerbo symlink                              # the tree follows: cerbo/archive/rust/ownership.md
cerbo folder create inbox                  # an empty folder that stays until deleted
cerbo folder order --root inbox archive    # inbox first, then archive, then the rest
cerbo folder list                          # the tree as the sidebar shows it
```

# SEE ALSO

- Project repository: https://github.com/antono/cerbo
- Tauri framework: https://tauri.app/
- Schema.org: https://schema.org/
- FOAF ontology: http://xmlns.com/foaf/spec/
- HackMD annotation syntax: https://hackmd.io/@docs/hackmd-syntax
