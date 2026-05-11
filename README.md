# Cerbo

**Cerbo** (/ˈt͡serbo/) is Esperanto for "brain". It is a local-first markdown wiki editor that treats the filesystem as a
first-class citizen. It stores all knowledge as plain files you fully own, with
`cerbo://` link-based graph navigation as the core interaction model.

## Core Features

- **Local-First & Private**: Your data stays on your machine in plain Markdown
  files and folders. No cloud dependency, no proprietary formats.
- **Desktop & CLI**: Work your way. Use the GUI for visual editing or the `cerbo`
  CLI for terminal workflows, automation, and headless indexing.
- **UUID-Based Storage**: Each page/object is a `.cerbo/objects/<uuid>/` folder,
  with no slug derivation or name-based paths.
- **Wikilink-Free**: Uses `cerbo://<uuid>` links instead of `[[Wikilinks]]`.
- **Backlinks & Annotations**: Automatic backlink tracking (`backrefs.ttl`) and
  HackMD-style annotations (`[Text]{prefix:Type}` → `annotations.ttl`).
- **Multiple Vaults**: Manage multiple isolated knowledge bases (vaults)
  anywhere on your disk.
- **XDG Compliant**: App configuration and caches follow the XDG Base Directory
  Specification.

## Tech Stack

- **Backend**: Rust Workspace (Core library + CLI + Tauri Desktop)
- **Frontend**: [Svelte 5](https://svelte.dev/) +
  [Carta](https://github.com/BearToCode/carta) (Markdown Editor) + Tailwind CSS
- **Package Manager**: [Bun](https://bun.sh/)
- **Environment**: [Nix Flakes](https://nixos.org/explore/#nix-flakes)

## Getting Started

This project uses [Nix Flakes](https://nixos.org/explore/#nix-flakes) to manage the development environment, including
Rust, Bun, and build dependencies.

### 1. Enter the Environment

You can enter the development shell using:

```bash
nix develop
```

Alternatively, if you have [direnv](https://direnv.net/) installed, just run `direnv allow` and the environment will be automatically loaded when you enter the directory.

### 2. Run Development Tasks

All development commands (bun, cargo) should be run inside the Nix development shell:

```bash
bun run tauri dev          # Start Tauri app with hot reload
cargo check                # Run Rust workspace type checks
cargo test -p cerbo-core   # Run core logic tests (use --test-threads=1 for isolation)
```

### 3. Build for Production

Use `nix build` to build the components:

```bash
nix build .#cerbo-desktop  # Build Desktop App
nix build .#cerbo          # Build CLI Binary
```

## Data Storage

- **Configuration**: `$XDG_CONFIG_HOME/cerbo/vaults.json` (defaults to
  `~/.config/cerbo/`). Stores only vault names and paths.
- **Cache**: `$XDG_CACHE_HOME/cerbo/` (defaults to `~/.cache/cerbo/`). Stores
  the link index and derived data.
- **Vaults**: Each vault has a `.cerbo/` directory containing:
  - `objects/<uuid>/` - UUID-based object storage (type: Product, Source, Attachment, Ontology)
  - `index.json` - Maps titles to UUIDs and vice versa
  - `ontology-map.json` - Maps prefixes (e.g., "schema") to ontology UUIDs

## Link Format

Cerbo uses `cerbo://<uuid>` links instead of wikilinks:

```markdown
Check out [this page](cerbo://123e4567-e89b-12d3-a456-426614174000) for details.
```

## HackMD Annotations

Cerbo supports HackMD-style semantic annotations:

```markdown
[Bob]{schema:Person} works at [Acme Corp]{schema:Organization}.
```

These are extracted to `annotations.ttl` in Turtle RDF format.

## Migration from Slug-Based Storage

Cerbo now uses UUID-based storage (`.cerbo/objects/<uuid>/`). This is a **breaking change** from the previous slug-based model.

To migrate an existing vault:

```bash
# Dry run - see what would be migrated
cargo run --package cerbo-migrate -- migrate --dry-run

# Actual migration
cargo run --package cerbo-migrate -- migrate

# Verify migration
cargo run --package cerbo-migrate -- verify
```

The migration tool will:
- Copy `page.md` to `.cerbo/objects/<uuid>/page.md`
- Copy `assets/` directory to `.cerbo/objects/<uuid>/assets/`
- Copy any other files in the page directory
- Generate new UUIDs for each page
- Create `meta.ttl` with object metadata

After migration, regenerate backlinks and annotations with:
```bash
cerbo index
```

## CLI Usage

### Initialize a Vault

Create a new vault in the current directory:

```bash
cerbo init
```

This creates a `.cerbo/` directory with the necessary structure and bundles standard ontologies (Schema.org, FOAF).

### Manage Pages

```bash
# Create a new page
cerbo page create "My First Page"
# Output: Created page: 550e8400-e29b-41d4-a716-446655440000

# List all pages
cerbo page list

# Read a page
cerbo page read 550e8400-e29b-41d4-a716-446655440000

# Write content to a page
cerbo page write 550e8400-e29b-41d4-a716-446655440000 "New content"

# Delete a page
cerbo page delete 550e8400-e29b-41d4-a716-446655440000
```

### Rebuild Metadata

The `cerbo index` command rebuilds metadata (backlinks and annotations) from page content. This is useful after:
- Manually editing `page.md` files outside of Cerbo
- Bulk imports or vault migrations
- If backlinks or annotations appear stale or missing
- To verify metadata integrity after crashes or interruptions

```bash
# Rebuild all metadata for the current vault
cerbo index

# Rebuild metadata for a single page (incremental)
cerbo index --page 550e8400-e29b-41d4-a716-446655440000

# Index a vault from outside its directory
cerbo index --vault /path/to/my/vault

# Get indexing statistics as JSON
cerbo index --json
```

The index command uses Git-style vault discovery: it walks up from the current directory looking for a `.cerbo/` directory. You can run it from any subdirectory within a vault.

### Import Content

```bash
# Import a webpage as a source object
cerbo import https://example.com/article

# Import an ontology
cerbo import-ontology https://schema.org/version/latest/schemaorg-current-https.jsonld
```

### Vault Management

```bash
# List all registered vaults
cerbo vault list

# Add a vault
cerbo vault add "Work Notes" /path/to/work/vault

# Remove a vault
cerbo vault remove <vault-id>

# Set active vault
cerbo vault active <vault-id>
```

### Utilities

```bash
# Resolve UUID to filesystem path
cerbo resolve 550e8400-e29b-41d4-a716-446655440000

# Show configuration
cerbo info
```

## License

LGPL-3.0-or-later
