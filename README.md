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

## License

LGPL-3.0-or-later
