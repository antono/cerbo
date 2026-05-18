## Why

Cerbo stores pages under opaque UUID directories (`.cerbo/objects/<uuid>/`), which is great for stable references but makes vaults unreadable to humans and other tools (file managers, editors, shell completion, search-everything indexers). Users need a way to browse and open notes by meaningful names without giving up UUID-based identity. The `symlink` command produces a parallel, human-readable view inside the same repository, and the desktop app will later reuse the same `virtualPath` tree for sidebar navigation.

## What Changes

- **Introduce the "cerbo vault" concept** (analogous to a git repository): any directory containing a `.cerbo/` subdirectory IS a cerbo vault. The directory is its root; everything cerbo-related — object storage, indexes, and the materialised symlink tree — lives inside it. This formalises terminology already implied by `cerbo init`. The term *vault* remains a valid synonym; `cerbo:vault` metadata is unchanged.
- Add `cerbo symlink` CLI command (no positional argument) that materialises a human-readable mirror of the current repository as relative symlinks pointing into the repository's own `.cerbo/objects/<uuid>/`.
- **Repository discovery is mandatory**: like `git`, the command walks up from cwd looking for `.cerbo/`. If none is found, the command aborts with `not a cerbo vault (or any parent up to mount point)`. There is no global-vault-name lookup mode — `cerbo symlink` always operates on the repository the user is currently inside.
- `.cerbo/` storage is the **single source of truth**, loosely analogous to the Nix store: each `.cerbo/objects/<uuid>/` directory is the canonical home of a page (the UUID is stable; the directory's contents are still freely edited), and the symlink tree is a derived view analogous to a Nix profile built by `buildEnv`. Every `symlink` run wipes the previous tree and rebuilds it from scratch from the current state of `.cerbo/objects/`. The Nix analogy is borrowed for the *projection algorithm* (real directories at merge points, symlinks at leaves, build-temp-then-atomic-swap), not for content-addressed immutability.
- **Tree-merging semantics (Nix profile-style)**: each segment of every page's `:virtualPath` becomes a real directory in the symlink tree, and pages sharing a `:virtualPath` are merged into the same directory. Only the leaf — the slug — is a symlink, and it points at the object directory `.cerbo/objects/<uuid>/` (not at `page.md`). Two pages can share `notes/rust/` as a real merged directory while their slug-named symlinks coexist inside it.
- **Conflict definition** follows the Nix profile model: a conflict exists if (a) two pages would produce the same leaf path (same `:virtualPath` + same `:slug`), or (b) one page's slug at a given level collides with another page's `:virtualPath` segment at the same level (i.e. the same path must be both a symlink and a real directory). Conflicts abort the rebuild and are reported with the offending UUIDs.
- **Output location**: the fixed path `<vault-root>/cerbo/`. The directory name is always literally `cerbo`, regardless of the repository's basename — predictable across all repos (muscle memory: `cd cerbo`), no redundant nesting like `my-notes/my-notes/`, trivial to `.gitignore`. The tree is always **inside** the repository, never in an arbitrary cwd.
- **Always relative symlinks**: because the materialised tree and `.cerbo/objects/` are guaranteed to live inside the same repository, every leaf symlink uses a relative path (e.g. `../../.cerbo/objects/<uuid>/`). The entire repository directory is therefore self-contained and portable — move it anywhere (USB drive, backup, different host) and the symlink tree continues to resolve.
- The tool will refuse to delete anything inside the output directory that is not a symlink it would have created (i.e. regular files / unrelated symlinks block the rebuild and are reported).
- Extend page metadata (`meta.ttl`) with two **separate, independent** RDF properties: `:slug` (string, e.g. `rust-ownership`) and `:virtualPath` (string, e.g. `notes/rust`). Both are stored as plain Turtle triples alongside existing metadata.
- Auto-generate `:slug` from `:title` on page creation if not already present; `:virtualPath` defaults to empty (page lives at the repository's symlink-tree root).
- Enforce uniqueness of the **combined** symlink target path (`<virtualPath>/<slug>`) repository-wide; collisions abort the rebuild and are reported (see Decision 6 in design.md — no silent auto-suffixing).
- Extend `cerbo index` to detect and report symlink-path collisions and missing `:slug` values; auto-backfill `:slug` for pages that lack one.
- Extend `cerbo init` to write a `.gitignore` entry of `/cerbo/` (creating the file if absent, appending if present and missing the line) so the derived symlink tree is never committed.
- Update the man page and `--help` output for the new command, the cerbo-repository terminology, the new metadata properties, and the `cerbo init` `.gitignore` behavior.

## Non-goals

- No bidirectional sync: the symlink tree is read-only metadata-wise. Editing the *content* of a page through its symlink edits the underlying `page.md` (the symlink is transparent), but renaming, moving, or deleting a symlink has NO effect on `.cerbo/` — the change is discarded on the next `symlink` run. Reverse-sync of structural edits belongs to a future change.
- No watcher: `symlink` is a one-shot rebuild. Live updates and incremental refresh are out of scope.
- No partial / incremental rebuild: there is no `--only <path>` mode. Every run is a full wipe-and-rebuild of the symlink tree.
- No web/HTTP exposure — strictly local filesystem symlinks.
- No attachment-specific naming rules beyond reusing the same `:slug` / `:virtualPath` mechanism if present; binary file naming preservation is out of scope here.
- No changes to `cerbo://<uuid>` link format or resolution semantics.

## Capabilities

### New Capabilities

- `vault-symlink`: defines the `cerbo symlink` command — mandatory cerbo-repository discovery (git-style walk-up), full-rebuild semantics with `.cerbo/` as the source of truth (Nix-store/profile analogue), Nix-profile-style tree merging of `:virtualPath` directories with `:slug` symlinks at the leaves, leaf-target convention (always-relative symlink → object directory, not `page.md`), conflict detection (leaf-vs-leaf and dir-vs-leaf), and safe-wipe rules. Output is always inside the vault root.

### Modified Capabilities

- `uuid-object-storage`: extend `meta.ttl` structure to include `:slug` and `:virtualPath` as optional-but-recommended properties; specify their semantics and the repository-wide uniqueness rule on their combination.
- `page-crud`: auto-generate `:slug` from `:title` when creating a page; allow callers to supply `:virtualPath` at creation time.
- `page-metadata-index`: during `cerbo index`, validate slug/virtualPath presence, backfill missing slugs, and report combined-path collisions.
- `vault-management`: formalise the "cerbo vault" definition (any directory containing `.cerbo/`) and the discovery rule (walk up from cwd to find one). `cerbo init` already creates one; this change names it. Additionally extend `cerbo init` to ensure `.gitignore` contains `/cerbo/` (create or append), so the materialised symlink tree is never committed.

## Impact

- **Code**: new `symlink` subcommand in `cli` crate; repository-discovery helper (walk-up for `.cerbo/`) in `core` crate; slug-generation + virtual-path helpers in `core`; meta.ttl serializer/parser updates for the two new properties; indexer extensions.
- **APIs**: `meta.ttl` gains two RDF predicates (additive, backwards-compatible — pages without them keep working). `core` gains a public `find_vault_root(cwd) -> Option<PathBuf>` helper, reusable by other commands.
- **Filesystem**: introduces a new directory `<vault-root>/cerbo/` inside the cerbo vault when `symlink` runs. Each run wipes and rebuilds the tree. Any regular file or non-cerbo symlink found inside it blocks the rebuild and is reported (no silent overwrite or deletion of user data). The repository directory remains self-contained: all materialised symlinks are relative paths into its own `.cerbo/objects/`.
- **Dependencies**: a small slug crate (e.g. `slug`, which transliterates via `deunicode`) and `pathdiff` for relative-path computation — to be confirmed in design.
- **Desktop app**: future consumer of `:virtualPath` for sidebar trees; this change unblocks that work but does not modify `cerbo-desktop`.
- **Docs**: man page, README CLI section, and `--help` strings, including the cerbo-repository terminology.
