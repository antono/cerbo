# NAME

cerbo - Semantic knowledge management tool for agents and humans

# SYNOPSIS

`cerbo [OPTIONS] <COMMAND>`

# DESCRIPTION

Cerbo is a local-first markdown wiki tool designed for both human and AI agent workflows. It uses a UUID-based storage model where all content objects are stored as directories named by UUID, making it easy to reference and link between objects.

The tool supports managing multiple vaults (workspaces), creating and editing pages with markdown content, importing web content as source objects, and working with ontologies (Schema.org, FOAF) for semantic markup using the HackMD annotation syntax `[Text]{prefix:Type}`.

# COMMANDS

## init

Initialize a new vault in the current directory. Creates a `.cerbo/` directory with the necessary structure and bundles standard ontologies (Schema.org, FOAF).

Usage: `cerbo init [OPTIONS]`

Options:
- `--json` - Output result as JSON

## vault

Vault management commands for working with multiple workspaces.

Usage: `cerbo vault <SUBCOMMAND>`

### Subcommands:

- `cerbo vault list [--json]` - List all registered vaults
- `cerbo vault add <NAME> <PATH> [--json]` - Add a new vault with given name at specified path
- `cerbo vault remove <ID> [--json]` - Remove a vault by its ID
- `cerbo vault active <ID> [--json]` - Set the active vault

## page

Page management commands for creating, reading, and modifying wiki pages.

Usage: `cerbo page <SUBCOMMAND>`

### Subcommands:

 - `cerbo page list [--json] [--vault <VAULT_ID>]` - List all pages in the specified vault or the current vault if omitted
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
    +-- vaults.toml           # Vault registry
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

# SEE ALSO

- Project repository: https://github.com/yourusername/cerbo
- Tauri framework: https://tauri.app/
- Schema.org: https://schema.org/
- FOAF ontology: http://xmlns.com/foaf/spec/
- HackMD annotation syntax: https://hackmd.io/@docs/hackmd-syntax
