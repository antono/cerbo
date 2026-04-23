# Cerbo

**Cerbo** (/ˈt͡serbo/) is Esperanto for "brain". It is a local-first markdown wiki editor that treats the filesystem as a
first-class citizen. It stores all knowledge as plain files you fully own, with
wikilink-based graph navigation as the core interaction model.

## Core Features

- **Local-First & Private**: Your data stays on your machine in plain Markdown
  files and folders. No cloud dependency, no proprietary formats.
- **Desktop & CLI**: Work your way. Use the GUI for visual editing or the `cerbo`
  CLI for terminal workflows, automation, and headless indexing.
- **Wikilink-Based Graph**: Navigate and link ideas naturally with
  `[[Wikilinks]]`.
- **Page-as-Folder Structure**: Each page is a `<slug>/page.md` folder, allowing
  you to co-locate assets (images, PDFs) directly with your notes.
- **Rename Cascade**: Renaming a page automatically updates all links to it
  across your entire vault.
- **Backlinks Panel**: See the context of your knowledge with a built-in
  backlinks explorer.
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
cargo test -p cerbo-core   # Run core logic tests
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
- **Vaults**: Pure markdown and assets. Cerbo does not store hidden metadata
  inside your vault directories.

## License

LGPL-3.0-or-later
