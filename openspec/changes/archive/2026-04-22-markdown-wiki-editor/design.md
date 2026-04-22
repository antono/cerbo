## Context

Cerbo is a new desktop application — no existing codebase to migrate. The design
must establish the foundational architecture for all subsequent work. Key
constraints from the proposal:

- Local-first: all data is plain files the user owns
- Vaults are self-contained and relocatable
- XDG Base Directory Specification for all app-managed paths
- Stack is fixed: Tauri v2, Svelte 5, Carta, shadcn-svelte, Bun, devenv.sh

## Goals / Non-Goals

**Goals:**
- Define the Tauri command surface (Rust ↔ Svelte boundary)
- Define the vault and page data model on disk
- Define slug derivation and wikilink resolution algorithms
- Define XDG path usage
- Define the link index: structure, build, invalidation

**Non-Goals:**
- UI layout and component design (belongs in specs)
- Full-text search
- Cross-vault linking
- Mobile or web targets
- Sync or collaboration

## Decisions

### D1: Project scaffold — `create-tauri-app` with SvelteKit + Bun

Use `bun create tauri-app` with the SvelteKit template as the project foundation.
This gives a conventional Tauri v2 project layout with Vite, TypeScript, and
SvelteKit routing out of the box. Bun is the package manager throughout.

**Alternatives considered:**
- Vanilla Svelte (no SvelteKit): SvelteKit's file-based routing maps naturally
  to vault/page navigation. Rejected vanilla for lack of routing.
- npm/pnpm: Bun has explicit Tauri support and faster installs. No reason to use
  alternatives.

---

### D2: Page model — folder-per-page with `page.md`

Each wiki page is a directory named after its slug, containing `page.md` and
optionally co-located assets.

```
<vault>/
  rust-ownership/
    page.md
    diagram.png
  my-tauri-app/
    page.md
    screenshot.png
```

**Rationale:** Co-location of assets with their page eliminates the need for a
shared asset pool, avoids path ambiguity in markdown image references, and keeps
the vault portable. Standard markdown `![](diagram.png)` works natively because
the renderer's base URL is the page folder.

**Alternatives considered:**
- Flat `.md` files per page: no asset co-location, harder to extend.
- Shared `assets/` folder at vault root: breaks portability, introduces naming
  conflicts across pages.

---

### D3: Slug derivation algorithm

Page titles are transformed to kebab-case folder names via a deterministic
algorithm:

1. Unicode normalize (NFKD)
2. Strip combining diacritics (ä → a, é → e)
3. Lowercase
4. Replace non-alphanumeric runs with `-`
5. Collapse and trim leading/trailing `-`

Examples:
- `"Rust Ownership"` → `rust-ownership`
- `"café & résumé"` → `cafe-resume`
- `"C++ Basics"` → `c-basics`
- `"My Tauri App!"` → `my-tauri-app`

The slug function lives in a shared Rust utility (called from Tauri commands) and
is also exposed to the Svelte frontend via a Tauri command for real-time feedback
during page creation.

**Alternatives considered:**
- Preserving `+` as `plus` etc.: adds complexity with little benefit for typical
  note titles.
- Allowing spaces in folder names: filesystem escaping headaches, rejected.

---

### D4: Wikilink resolution

`[[Title]]` wikilinks resolve within the current vault only.

Resolution algorithm:
1. Normalize the link text using the slug algorithm → `target-slug`
2. Look up `target-slug` in the vault's page index
3. If found → navigate to `<vault>/<target-slug>/page.md`
4. If not found → offer to create the page

Resolution is case-insensitive: `[[rust ownership]]`, `[[Rust Ownership]]`, and
`[[rust-ownership]]` all resolve to `rust-ownership/page.md`.

**Alternatives considered:**
- Full-path wikilinks (`[[vault/page]]`): cross-vault linking adds complexity,
  deferred indefinitely.
- Fuzzy matching: rejected for v1 — deterministic slug match is predictable.

---

### D5: Vault registry — `vaults.json`

The vault registry is stored at `$XDG_CONFIG_HOME/cerbo/vaults.json`.

Schema:
```json
{
  "vaults": [
    {
      "id": "<uuid-v4>",
      "name": "Work",
      "path": "/home/me/work-wiki"
    }
  ],
  "activeVaultId": "<uuid-v4>"
}
```

The `id` field is a stable UUID generated once at `vault_add()` time. It persists
across vault renames and path changes, serving as the cache key.

**Rationale:** Storing only name, path, and ID keeps the config free of
vault-specific data. The vault itself remains a clean directory of markdown files.

---

### D6: XDG path layout

| Purpose            | Path                                    |
|--------------------|-----------------------------------------|
| Vault registry     | `$XDG_CONFIG_HOME/cerbo/vaults.json`    |
| Link index cache   | `$XDG_CACHE_HOME/cerbo/<vault-id>/index.json` |

Tauri's `app_config_dir()` on Linux returns `$XDG_CONFIG_HOME/<app>` and
`app_cache_dir()` returns `$XDG_CACHE_HOME/<app>`, so XDG compliance comes from
using Tauri's path APIs throughout — no manual path construction.

---

### D7: Link index — build and invalidation

The link index maps each page to its outbound wikilinks and is used to compute
backlinks.

Structure (`$XDG_CACHE_HOME/cerbo/<vault-id>/index.json`):
```json
{
  "version": 1,
  "builtAt": "<iso-timestamp>",
  "pages": {
    "rust-ownership": {
      "title": "Rust Ownership",
      "links": ["rust", "memory-model"]
    }
  }
}
```

Backlinks for a page are computed on demand by inverting the index (not stored).

**Build strategy:**
- Built on vault open if cache is missing or stale
- Updated incrementally via `notify` (FS watcher) on `page.md` save events
- Invalidated entirely on rename-cascade (full rebuild)

**Alternatives considered:**
- In-memory only (no cache): acceptable for small vaults, but rebuild on every
  launch is wasteful. Cache with FS watcher gives best of both.
- SQLite: overkill for a flat JSON structure of this size.

---

### D8: Rename cascade — atomic find-replace

When a page is renamed (`old-slug` → `new-slug`):

1. Rename the folder on disk: `old-slug/` → `new-slug/`
2. Scan all `*/page.md` files in the vault for `[[OldTitle]]` patterns
3. Replace all matches with `[[NewTitle]]` in a single Rust pass
4. Rebuild link index

The scan uses case-insensitive matching against the old title and its slug form.
The operation is best-effort atomic: folder rename happens first, then file
updates. A failure mid-cascade leaves the vault in a readable state (links just
become broken temporarily).

---

### D9: Asset serving — Tauri asset protocol

Assets in page folders are served to the Svelte renderer via Tauri's custom
`asset://` protocol, which requires registering allowed paths in `tauri.conf.json`.
The vault root is registered as an allowed asset scope on vault open.

This means `![](diagram.png)` in `rust-ownership/page.md` renders correctly
because Carta's renderer resolves relative URLs against the page folder's
`asset://` base.

---

### D10: devenv.sh setup

`devenv.sh` manages all native tooling:

- `rustup` + stable Rust toolchain
- `bun` (package manager + JS runtime)
- `cargo-tauri` (Tauri CLI)
- Shell scripts: `dev` (tauri dev), `build` (tauri build), `check` (cargo check)

No system Rust or Node installation required. `devenv shell` gives a fully
reproducible build environment.

## Risks / Trade-offs

- **Carta wikilink plugin is custom work** → Mitigation: Carta has a well-documented
  plugin API; the plugin is scoped to syntax highlighting + click handler, not a
  full language server.

- **Rename cascade is not transactional** → Mitigation: Failure mid-cascade leaves
  broken links but no data loss. A future improvement could use a write-ahead log.

- **FS watcher (`notify`) may miss rapid saves** → Mitigation: On vault open,
  always compare cache `builtAt` against vault `mtime` and rebuild if stale.

- **Large vaults (1000+ pages) may have slow index builds** → Mitigation: Index
  build is async and non-blocking; UI shows stale data until rebuild completes.
  Search is explicitly out of scope for v1.

- **Tauri asset protocol scope must be updated on vault add** → Mitigation: Call
  `allow_directory` on the new vault path at `vault_add()` time; document this as
  a required step in the Tauri command implementation.

## Open Questions

- Should `vaults.json` store `activeVaultId` or should last-active vault be
  restored from a separate state file? (Minor — decide at implementation time.)
- Carta wikilink plugin: autocomplete on `[[` trigger — should it show all pages
  or filter as user types? (UX detail — decide in wikilink-editing spec.)
