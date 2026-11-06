# Design

## Context

`add-virtual-path-tree` turned the sidebar into a tree built from each page's
`cerbo:virtualPath`. Folders are derived from paths (its design D1), siblings
sort folders first and then by title, and the only persistent tree state is
the per-machine collapse set in `ui.toml`. That change also replaced native
HTML5 drag-and-drop with pointer-event drag (its D8), because a `draggable`
row never starts a drag on GNOME Wayland.

This change adds a user-defined order and folders that exist without pages.
Both are properties of the vault, not of one machine: a vault synced to a
second computer should show the same tree.

## Goals / Non-Goals

**Goals:**

- Any row can be placed at any position among its siblings, by drag, by
  keyboard and by menu.
- Folders can be created empty, moved by drag, and deleted when empty.
- One small, human-readable vault file holds all of it, and the CLI can
  read and change it.
- A vault without the file looks exactly as today.

**Non-Goals:**

- Ordering in `cerbo page list` or `cerbo symlink`.
- Order in RDF or SPARQL.
- Live reload of the desktop tree on external edits of the file.

## Decisions

### D1. One vault file: `.cerbo/tree.toml`

```toml
# Sidebar layout, written by Cerbo. Safe to edit by hand.
pinned = ["inbox", "projects/2026"]

[order]
"" = ["inbox/", "6f1c2d3e-0000-4000-8000-000000000137", "notes/"]
"notes" = ["rust/", "1b2c3d4e-0000-4000-8000-000000000137"]
```

- `pinned`: folder paths that exist even with no page under them. A pinned
  path also brings its ancestors into the tree.
- `order`: for a folder path (`""` is the root), its children in display
  order. A child page is its UUID; a child folder is its last path segment
  followed by `/`. A segment can never contain `/`, so the two forms never
  clash.
- Written with `fsio::write_atomic_str`, keys sorted (`BTreeMap`), pinned
  paths sorted and deduplicated, so diffs stay small.
- Alternatives rejected: a `cerbo:position` triple in each `meta.ttl` gives
  folders nowhere to live and makes one drag rewrite many objects; `ui.toml`
  is per machine and invisible to the CLI.

### D2. Display order rule

The children of a folder are its existing child folders and pages. They show
in this order:

1. Every child named in `order[folder]`, in that order.
2. Every other child, in today's default order: folders first, then pages,
   each by name or title, case-insensitive (`localeCompare` with
   `sensitivity: 'base'` in the frontend, Unicode lowercase in Rust).

An entry that names no existing child (a deleted page, a page that moved
away, a folder that is gone) is skipped. Duplicates count once, at their
first position. The rule lives twice, in `core/src/tree.rs` for
`cerbo folder list` and in `src/lib/page-tree.ts` for the sidebar, and both
are tested against the same cases.

### D3. Keeping the file consistent

- **Page move** (`page_update_virtual_path`): does not touch the file. The
  UUID left in the old folder's list is skipped by D2; if the page moves back
  it regains its old place. The desktop writes the new folder's list itself
  after a drag (D6).
- **Folder rename** (`folder_rename`, also used for a folder drag): after the
  page writes, rewrites the layout: every `pinned` path and `order` key equal
  to the old prefix or under it gets the new prefix; the `old-name/` entry in
  the old parent's list is replaced in place when the parent is unchanged, or
  removed when the folder moves to another parent. A rename that matches no
  page but a pinned folder is no longer a no-op: it renames the pinned
  folder.
- **Folder delete**: allowed only when no page lives at or under the path.
  Removes pinned paths at or under it, `order` keys at or under it, and its
  entry in the parent's list.
- **Writes prune:** every write of a list drops entries that name no existing
  child, so stale UUIDs do not pile up in lists that are edited.

### D4. Core API (`core/src/tree.rs`)

```rust
pub struct TreeLayout { pub pinned: Vec<String>, pub order: BTreeMap<String, Vec<String>> }

pub fn tree_layout(ctx: &CerboContext) -> Result<TreeLayout, String>;
pub fn folder_create(ctx: &CerboContext, path: &str) -> Result<(), String>;
pub fn folder_delete(ctx: &CerboContext, path: &str) -> Result<(), String>;
pub fn folder_set_order(ctx: &CerboContext, parent: &str, entries: Vec<String>) -> Result<(), String>;
pub fn folder_tree(ctx: &CerboContext) -> Result<Vec<TreeNode>, String>; // D2 applied, for the CLI
```

- Paths are validated with `vault::validate_virtual_path`; `""` means the
  root where a parent is expected.
- `folder_create` fails when the folder already exists (derived or pinned):
  "folder already exists: notes".
- `folder_delete` fails with "folder is not empty: notes (3 pages)".
- `folder_set_order` rejects an entry that is neither a UUID nor `name/`
  with a valid segment; it does not require entries to exist (prune applies).
- A missing file reads as an empty layout. A file that does not parse is an
  error naming the file; nothing overwrites it.
- `folder_rename` in `page.rs` calls a `tree::rename_prefix` helper after its
  page writes.

### D5. Tauri commands and CLI

- Tauri: `tree_layout()`, `folder_create(path)`, `folder_delete(path)`,
  `folder_set_order(parent, entries)`, all through `off_main` like the other
  page commands.
- CLI under the existing `cerbo folder`:
  - `cerbo folder list [--json]`: the ordered tree, two spaces per level,
    folders with a trailing `/`, pages as `Title  <uuid>`.
  - `cerbo folder create <PATH> [--json]`
  - `cerbo folder delete <PATH> [--json]`
  - `cerbo folder order <PATH|--root> <ENTRY>... [--json]`: an entry is a
    page UUID or a child folder name (with or without the trailing `/`).

### D6. Placing a row (frontend)

A drop or a keyboard move becomes a placement: the row, the target parent
folder, and the index among the parent's children. `page-tree.ts` gains a
pure `placement(tree, row, parent, index)` that returns the store calls:

1. When the parent changes: `page_move` for a page, `folder_rename` for a
   folder (old path to `parent/name`).
2. Always: `folder_set_order(parent, keys)` with the parent's children in
   the new order, as entry keys (D1).

Both run inside the existing `vaultWrite` queue, then the page list and the
layout reload once.

### D7. Drop zones

The row under the pointer and the pointer's height inside it decide:

| Row | Top quarter | Middle half | Bottom quarter |
| --- | --- | --- | --- |
| Folder, collapsed or empty | before | inside, last | after |
| Folder, expanded with children | before | inside, last | inside, first |
| Page | before (top half) | | after (bottom half) |

"Before" and "after" place the row at that position in the target row's
parent; a 2 px line is drawn at the row edge, indented to the target depth.
"Inside" highlights the folder row. The root zone below the list means
"root, last". A drop that would put a folder inside itself or a descendant is
refused (no indicator, no call); dropping a row where it already is does
nothing. The drag starts after 4 px of movement, Escape cancels it, and the
click that ends a drag does not open the row.

### D8. Keyboard and menus

- `Alt+ArrowUp`/`Alt+k` and `Alt+ArrowDown`/`Alt+j` on a focused row swap it
  with its previous or next sibling; at either end nothing happens. Focus
  stays on the moved row. `Alt+ArrowLeft`/`Right` stay Back and Forward.
- Row menu: "Move up" and "Move down" on every row (disabled at the ends).
  Folder menu: "New page here", "New folder here", "Rename folder",
  "Delete folder" (only when no page is under it).
- Right-click on sidebar space that is not a row opens the same menu
  component with "New page" and "New folder", both at the root.
- `NewFolderDialog.svelte` reuses `VirtualPathInput` (`#new-folder-path`)
  with folder suggestions; "New folder here" prefills `parent/`.

## Risks / Trade-offs

- [The order rule exists in Rust and TypeScript] → Small rule, shared test
  cases in both suites; the frontend never writes an order it cannot read.
- [Two writes per cross-folder drop (move, then order)] → Both are atomic;
  if the second fails the page is in the right folder at its default
  position, and the error banner says so.
- [Hand edits can break the file] → Parse errors are shown, not overwritten;
  `cerbo folder list` reports them too.
- [Stale entries after page deletes] → Skipped on read, pruned on the next
  write of that list.
- [Folder drag rewrites every page under it] → Same per-page writes and
  failure report as `folder rename`.

## Migration Plan

None: the file is optional and created on the first reorder or folder
creation. Rollback to an older build ignores it; pinned empty folders then
simply do not show.

## Open Questions

None.
