# Design

## Context

See proposal.md for motivation. What exists today:

- `ObjectMeta.virtual_path: Option<String>` is read from and written to
  `meta.ttl`; `vault::validate_virtual_path` rejects leading/trailing slashes,
  empty and dot segments and NUL bytes. `cerbo symlink` and
  `metadata_index::detect_path_collisions` already consume it.
- `page_update_title` is the template for a single-object metadata update:
  read `meta.ttl`, mutate, `write_to_file` (atomic), then
  `object::sync_index(ctx, uuid)` pushes the object's graph into the semantic
  index.
- `page::PageMeta` is `{uuid, title}`; `page_list` already parses `meta.ttl`
  for every page, so adding the path costs nothing.
- `PageList.svelte` is a flat `<ul>`; keyboard handling lives in one
  `onkeydown` on the wrapper and indexes into `app.pages` by button order.
- `NewPageDialog.svelte` has a path field with a hardcoded suggestion list and
  a frontend validator (`^[a-z0-9\-_\/]+$`) stricter than core.
- `UiSettings` (`ui.toml` in the config dir) is app-wide; nothing in it is
  keyed by vault yet.
- The frontend has no context menu or drag-and-drop code to reuse.

## Goals / Non-Goals

**Goals:**

- One core function per operation, shared by CLI and Tauri, with the same
  error strings.
- The tree is a pure function of `app.pages`; no second source of truth.
- The flat-list keyboard behaviour keeps working when a vault has no paths.

**Non-Goals:**

- A generic tree component or virtualised rendering. A vault of 137 pages
  renders fine as plain rows; revisit above a few thousand.
- Watching the vault for external `meta.ttl` edits (no watcher exists in the
  desktop today).

## Decisions

### D1. Folders are derived, never stored

The tree is built client-side from `virtualPath` strings. A folder is the set
of pages whose path equals or starts with it. Consequences: no empty folders,
no folder metadata file, folder rename is a batch page move.

Alternative: a `folders.ttl` or per-folder object. Rejected: it would add a
second thing to sync, migrate and keep consistent with `cerbo symlink`, which
already treats paths as the only truth.

### D2. Core API

```rust
// core/src/page.rs
pub fn page_update_virtual_path(ctx, uuid: String, path: Option<String>) -> Result<(), String>
pub fn folder_rename(ctx, old_prefix: &str, new_prefix: &str) -> Result<FolderRenameReport, String>
pub struct FolderRenameReport { pub changed: Vec<String>, pub failed: Vec<(String, String)> }
```

- `None` or `Some("")` clears the predicate. Validation runs before any
  write. Source objects are allowed: `page_write` refuses Source because it
  guards content; placement is metadata.
- `folder_rename` lists pages via `page_list`, filters by prefix, and calls
  `page_update_virtual_path` per page. It never aborts mid-way; failures are
  collected and reported. After the loop it calls
  `semantic::sync_objects(vault, &changed)` once instead of once per page.
- `PageMeta` gains `virtual_path: Option<String>` serialised as
  `virtualPath`. An invalid stored value is already dropped by
  `ObjectMeta::read_from_file`, so `page_list` reports `null` for it for free.

### D3. Tauri commands

`page_move(uuid, virtualPath: Option<String>)` and
`folder_rename(oldPrefix, newPrefix)` in `src-tauri/src/page.rs`, both via
`off_main`. The report is returned as JSON; the UI shows `failed` entries in
the error banner.

### D4. CLI surface

- `cerbo page move <UUID> [PATH] --root` with a clap `ArgGroup` making exactly
  one of `PATH`/`--root` required.
- `cerbo folder rename <OLD> <NEW>` as a new top-level `folder` command rather
  than `page folder-rename`: it acts on many pages and reads naturally.
- `cerbo page list` text output appends ` [path]`; JSON adds `virtualPath`.
  Scripts parsing `<uuid>: <title>` by splitting on the first `: ` keep
  working; only the title tail changes.

### D5. Tree model in the frontend

A derived store `pageTree` computed with `$derived` from `app.pages`:

```ts
type TreeNode =
  | { kind: 'folder'; path: string; name: string; depth: number; children: TreeNode[] }
  | { kind: 'page';   path: string; uuid: string; title: string; depth: number };
```

Rendering flattens it to visible rows (respecting collapsed folders) so the
existing keyboard handler can keep working on a flat array of row elements.
Each row carries `data-kind` and `data-path`/`data-uuid` attributes, which the
keyboard and drag handlers read instead of indexing into `app.pages`.

Alternative: a recursive Svelte component. Rejected for keyboard handling;
flattening keeps focus movement a simple index step.

### D6. Expansion state

`app.collapsedFolders: Set<string>` keyed by folder path, stored as
`collapsedFolders[vaultId] = ["notes", "archive/2025"]` in `ui.toml`. Only
collapsed folders are stored, so a new folder is expanded by default and
nothing accumulates when folders disappear (stale entries are pruned on save).

`openPage` removes every ancestor path of the opened page from the set.

Alternative: store in `state.toml` next to `lastOpenPage`. Rejected: that file
is vault-state used by the CLI too; collapse state is pure desktop UI.

### D7. Move dialog reuses the New Page path field

Extract the path input (field, validation message, suggestions dropdown) from
`NewPageDialog.svelte` into `VirtualPathInput.svelte`. Suggestions come from a
derived `folderPaths` list (every distinct folder prefix in `app.pages`), not
from the hardcoded array. The frontend validator is replaced by a port of
`validate_virtual_path` so the two never disagree; the backend remains the
authority and its message wins when they do.

`MovePageDialog.svelte` and `RenameFolderDialog.svelte` follow
`RenamePageDialog.svelte`: one field, Enter/Escape, error line, driven by
`app.moveUuid` / `app.renameFolderPath` state like `renameUuid`.

### D8. Context menu and drag-and-drop

- Context menu: a small `RowMenu.svelte` positioned at the pointer, closed on
  outside click or Escape. No library.
- Drag-and-drop: native HTML5 DnD (`draggable`, `dragover`, `drop`) with the
  page UUID in `dataTransfer`. Folder rows and a root drop zone at the bottom
  of the list are the only targets. Dropping onto a folder calls `page_move`
  with that folder's path; dropping onto the page's own folder is a no-op.
  Native DnD was chosen over pointer-event emulation because the tree is
  small and the only drop targets are rows.

### D9. Keyboard

`h`/`l` and `ArrowLeft`/`ArrowRight` are added to the existing sidebar
handler. `m` joins `r` and `Delete` as a row action. `KeyboardHelp.svelte`
lists them. In preview mode the layout-level handler still owns navigation,
as today.

## Risks / Trade-offs

- [Folder rename is not atomic across pages] → Per-page atomic writes plus a
  report of failures; the user can re-run the rename on the leftover prefix.
  `cerbo index` reports any resulting collision.
- [Slug collision after a move] → Not blocked (same policy as creation); the
  tree shows both pages, `cerbo index`/`cerbo symlink` surface it.
- [Native DnD feels inconsistent across WebKit/Windows] → DnD is a convenience
  on top of the Move dialog, which is the primary and keyboard-reachable path.
- [`page_list` on every tree refresh re-reads 137 `meta.ttl` files] → Same as
  today; the list is reloaded only after mutations and on vault open.
- [Prefix match on `notes` vs `notebook`] → Match is `== prefix` or
  `starts_with(prefix + "/")`, covered by spec scenarios.

## Migration Plan

No data migration: existing `cerbo:virtualPath` values render as a tree on
first launch. `ui.toml` gains an optional `collapsedFolders` table; older
builds ignore it. Rollback is a plain downgrade.

## Open Questions

- Whether `cerbo page list` text output should hide the path when no page in
  the vault has one. Default: always print the suffix only when present, so
  nothing changes for flat vaults.
