## Why

Existing note-taking and wiki tools are either cloud-dependent, lack proper
knowledge graph linking, or don't treat the filesystem as a first-class citizen.
**Cerbo** is a local-first markdown wiki editor that stores all knowledge as
plain files the user fully owns, with wikilink-based graph navigation as the
core interaction model.

## What Changes

- New Tauri desktop application (Rust + Svelte)
- Markdown editing via Carta editor with wikilink plugin
- Multiple isolated vaults (any folder on disk)
- Page-as-folder structure: each page is `<slug>/page.md` with co-located assets
- Wikilink resolution: `[[Page Title]]` → `slug(title)/page.md`
- Backlinks panel: shows all pages linking to the current page
- Automatic slug derivation from page title (kebab-case, unicode-normalized)
- Rename cascade: renaming a page updates all `[[wikilinks]]` across the vault
- devenv.sh manages Rust, Bun, and build tooling
- App config and data follow XDG Base Directory Specification (Linux)

## Capabilities

### New Capabilities

- `vault-management`: Add, remove, and switch between multiple vault folders.
  Vault registry (`$XDG_CONFIG_HOME/cerbo/vaults.json`) stores only name and
  path per vault. Each vault is self-contained and relocatable — no
  vault-specific data stored outside the vault folder.
- `page-crud`: Create, read, update, delete pages. Each page is a folder
  (`<slug>/`) containing `page.md` and any assets. Page title drives slug
  generation.
- `slug-resolution`: Derive kebab-case folder names from page titles. Normalize
  unicode, strip special characters. Resolve `[[Title]]` wikilinks to folder
  slugs case-insensitively.
- `wikilink-editing`: Carta plugin for `[[wikilink]]` syntax — autocomplete
  existing pages, highlight broken links, click-to-navigate.
- `backlinks`: Parse all pages in the vault to build a link index. Display
  backlinks panel for the current page.
- `rename-cascade`: When a page is renamed, update its folder name and
  find-replace all `[[OldTitle]]` occurrences across the vault atomically.
- `asset-management`: Assets (images, PDFs, etc.) stored inside the page folder
  alongside `page.md`. Served to the renderer via Tauri asset protocol. Standard
  markdown image syntax works natively.
- `devenv-setup`: devenv.sh configuration covering Rust toolchain, Bun, Tauri
  CLI, and build/dev scripts.
- `xdg-dirs`: App config stored according to XDG Base Directory Specification.
  Vault registry in `$XDG_CONFIG_HOME/cerbo/vaults.json` (default
  `~/.config/cerbo/`). Caches (link index etc.) in `$XDG_CACHE_HOME/cerbo/`
  (default `~/.cache/cerbo/`), keyed by vault ID. Config contains only vault
  names and paths — nothing vault-specific. Each vault is fully self-contained
  and relocatable — no hidden folders inside vault directories.

### Modified Capabilities

## Impact

- New project — no existing code affected
- Dependencies: Tauri v2, Svelte 5, Carta, shadcn-svelte, devenv.sh
- App config: `$XDG_CONFIG_HOME/cerbo/vaults.json` — vault names, paths, and
  stable IDs only
- Cache: `$XDG_CACHE_HOME/cerbo/<vault-id>/` — link index and derived data;
  keyed by vault ID so cache survives vault relocation
- Vault folders: pure markdown and assets, no hidden metadata
- Rust crates: `tauri`, `notify` (FS watching), `serde`, `walkdir`
