# Proposal

## Why

`cerbo:virtualPath` already exists in `meta.ttl`, is validated, and drives the
`cerbo symlink` tree, but the desktop app ignores it: the sidebar is a flat
list, and once a page is created nothing (CLI or desktop) can change its path.
A vault with 137 pages is unusable as a flat list; users need the hierarchy
they already declare, and a plain way to edit it.

## What Changes

- **Sidebar tree:** the page list becomes a tree. Each `virtualPath` segment
  is a folder; pages with no path sit at the root. Folders are derived from
  page metadata, never stored: a folder exists while at least one page lives
  under it. Expand/collapse state persists per vault in UI settings.
- **Move page:** a "Move to…" action on every page (hover button, context
  menu, `m` in the sidebar) opens a dialog with a path field, autocomplete of
  existing folders and the same validation as the New Page dialog. Empty
  means root.
- **Drag and drop:** dragging a page onto a folder or onto the root area sets
  its `virtualPath` to that folder. The drop target is highlighted; invalid
  drops are refused.
- **Folder actions:** rename a folder (rewrites that prefix on every page
  under it, including nested folders) and "New page here" (opens the New Page
  dialog with the path prefilled).
- **Core and Tauri:** `page_update_virtual_path(uuid, path)` and
  `folder_rename(old_prefix, new_prefix)` in `cerbo-core`, exposed as Tauri
  commands. `page_list` entries gain `virtualPath` (null when absent).
- **CLI:** `cerbo page move <UUID> <PATH|--root>`, `cerbo folder rename <OLD>
  <NEW>`, and `cerbo page list` shows each page's path (also in `--json`).
  Man page updated.
- **Keyboard:** `j`/`k`/arrows walk the visible tree rows; `h`/`l` and
  `ArrowLeft`/`ArrowRight` collapse and expand folders.

Not **BREAKING**: `page_list` only adds a field; existing paths stay valid.

## Capabilities

### New Capabilities

- `page-virtual-path`: changing a page's `cerbo:virtualPath` after creation
  and renaming a folder prefix across pages: validation, atomic per-object
  writes, semantic index refresh, CLI and Tauri entry points.
- `desktop-page-tree`: the sidebar tree: folder derivation, expand/collapse
  persistence, move dialog, drag and drop, folder rename, new page in folder.

### Modified Capabilities

- `page-crud`: "List pages" entries include `virtualPath`.
- `sidebar-navigation`: navigation operates on visible tree rows and gains
  collapse/expand keys.
- `cli-tooling`: new `cerbo page move` command.

## Non-goals

- Empty folders. A folder with no pages does not exist and cannot be created
  on its own.
- Changing `cerbo:slug` or page titles from the tree. Rename page stays as is.
- Deleting a folder with its pages (bulk delete).
- Reordering pages inside a folder (`cerbo:position`), sorting other than by
  title.
- Changes to the `cerbo symlink` output format. It already follows
  `virtualPath`.
- Multi-select moves.

## Impact

- **Core:** `core/src/page.rs` (new update functions), `core/src/page.rs`
  `PageMeta` gains `virtual_path`; `core/src/vault.rs` validation reused.
  Semantic index must pick up the `meta.ttl` change for the moved page.
- **Tauri:** `src-tauri/src/page.rs` new commands, registered in `lib.rs`.
- **CLI:** `cli/src/main.rs`, `cli/man/cerbo.md`, `cli/tests/cli_integration.rs`.
- **Frontend:** `src/lib/PageList.svelte` rewritten as a tree, new
  `MovePageDialog.svelte`, `stores.svelte.ts` (`PageMeta`, expanded-folder
  state, `movePage`, `renameFolder`), `KeyboardHelp.svelte`,
  `core/src/ui_settings.rs` (expanded folders per vault).
- **Specs:** `vault-symlink` is unchanged; a path collision after a move is
  still surfaced by `cerbo index`.
