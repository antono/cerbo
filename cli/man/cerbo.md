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

Initialize a new vault in the current directory. Creates a `.cerbo/` directory with the necessary structure, bundles standard ontologies (Schema.org, FOAF), and adds `/cerbo/` to `.gitignore`.

Usage: `cerbo init [OPTIONS]`

Options:
- `--json` - Output result as JSON

## vault

Vault management commands for working with multiple workspaces.

Usage: `cerbo vault <SUBCOMMAND>`

### Subcommands:

- `cerbo vault list [--json]` - List all registered vaults. Auto-registered vaults are shown with an `(auto)` marker. JSON output includes an `is_auto` boolean field per vault.
- `cerbo vault add <NAME> <PATH> [--json]` - Add a new vault with given name at specified path
- `cerbo vault remove <ID> [--json]` - Remove a vault by its ID (works on both manually and auto-registered vaults)
- `cerbo vault active <ID> [--json]` - Set the active vault
- `cerbo vault approve <ID> [--json]` - Promote an auto-registered vault to the manual registry. Moves the entry from `vaults.auto.toml` to `vaults.toml`. Errors if the ID is not in the auto registry or the path is already manually registered.

## page

Page management commands for creating, reading, and modifying wiki pages.

Usage: `cerbo page <SUBCOMMAND>`

### Subcommands:

- `cerbo page list [--json] [--vault <ID>]` - List all pages in the current vault (auto-discovered from CWD, or override with --vault)
- `cerbo page create <TITLE> [--json]` - Create a new page with the given title
- `cerbo page read <UUID> [--json]` - Read the content of a page by its UUID
- `cerbo page write <UUID> <CONTENT> [--json]` - Write content to a page
- `cerbo page delete <UUID> [--json]` - Delete a page by its UUID

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

Import an ontology file and create a type:Ontology object.

Usage: `cerbo import-ontology <URL> [--json]`

Arguments:
- `URL` - The URL of the ontology file (e.g., Schema.org or FOAF)

Options:
- `--json` - Output result as JSON with uuid and url fields

## index

Rebuild metadata (backrefs.ttl, annotations.ttl) from page content. Used for recovery after manual edits, bulk imports, or vault migration.

Usage: `cerbo index [OPTIONS]`

The command discovers the vault using Git-style directory traversal: it walks up from the current directory looking for a `.cerbo/` directory. You can also specify an explicit vault path with `--vault`.

Options:
- `--vault <PATH>` - Explicit vault path (overrides Git-style discovery)
- `--page <UUID>` - Index only a single page (incremental mode)
- `--no-backfill-slug` - Skip backfilling missing `cerbo:slug` values into `meta.ttl`
- `--json` - Output result as JSON with pages_processed, links_found, annotations_found, slugs_backfilled, path_errors, collisions, errors

**Behavior:**
- Without `--page`: Performs full vault rebuild (two-pass: clear all backrefs, then rebuild)
- With `--page <UUID>`: Incremental indexing of a single page (faster, preserves other pages' metadata)
- Idempotent: Running twice produces identical results
- Handles corrupted/missing files gracefully (logs errors, continues processing)

**When to use:**
- After manually editing `page.md` files outside of Cerbo
- After bulk imports or vault migrations
- If backlinks or annotations appear stale or missing
- To verify metadata integrity after crashes or interruptions

## symlink

Build (or rebuild) a human-readable symlink tree at `<repo-root>/cerbo/`. Each page with a `cerbo:slug` value is exposed as a relative symlink named after its slug. Pages that declare a `cerbo:virtualPath` are nested inside corresponding subdirectories. The tree is rebuilt atomically: a new staging directory is written and then swapped into place so readers always see a consistent state.

Usage: `cerbo symlink [OPTIONS]`

The command discovers the vault using Git-style directory traversal: it walks up from the current directory looking for a `.cerbo/` directory. You can also specify an explicit vault path with `--vault`.

Options:
- `--vault <PATH>` - Explicit vault path (overrides Git-style discovery)
- `--json` - Output result as JSON with objects_scanned, leaves_created, dirs_created

**Behavior:**
- Symlinks are always relative (portable when the vault is moved or mounted elsewhere)
- Pages without a `cerbo:slug` are skipped (run `cerbo index` first to backfill slugs)
- Detects leaf-vs-leaf and dir-vs-leaf collisions and aborts with an error listing all conflicts
- Refuses to wipe `cerbo/` if it contains entries that were not created by cerbo (safe-wipe guard)
- `cerbo init` adds `/cerbo/` to `.gitignore` automatically

**When to use:**
- After `cerbo init` to create the initial symlink tree
- After creating or renaming pages (to refresh the tree)
- After moving the vault to a new location (symlinks are relative, so re-running fixes any stale links)

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
.vault-path/
  .cerbo/
    +-- objects/
    |   +-- <uuid>/
    |       +-- page.md        # Markdown content (for pages)
    |       +-- meta.ttl       # RDF metadata (Turtle format)
    |       +-- backrefs.ttl   # Backlinks to this object
    |       +-- annotations.ttl # HackMD-style annotations
    +-- index.json             # Object index
    +-- vaults.toml           # Vault registry (manually registered)
    +-- vaults.auto.toml      # Auto-registered vaults (CWD discovery)
    +-- ontology-map.json      # Ontology prefix mappings
    +-- ui.toml               # UI settings
    +-- state.toml            # Application state
```

## Object Types

- **Product** - Default type for pages created by users
- **Source** - Imported URLs (read-only references)
- **Attachment** - File attachments (future)
- **Ontology** - Ontology definitions (Schema.org, FOAF bundled by default)

## Links and Annotations

Links between objects use the format: `cerbo://<uuid>`

Annotations use HackMD syntax: `[Text]{prefix:Type}` where prefix maps to an ontology UUID via `ontology-map.json`.

Example: `[Alice]{foaf:Person}` - references the FOAF Person type

# EXAMPLES

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

Show configuration info:
```
cerbo info
```

Rebuild metadata for all pages in current vault:
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

Build symlink tree:
```
cerbo symlink
```

# SEE ALSO

- Project repository: https://github.com/yourusername/cerbo
- Tauri framework: https://tauri.app/
- Schema.org: https://schema.org/
- FOAF ontology: http://xmlns.com/foaf/spec/
- HackMD annotation syntax: https://hackmd.io/@docs/hackmd-syntax
