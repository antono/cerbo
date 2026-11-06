# Proposal

## Why

The sidebar tree from `add-virtual-path-tree` sorts every folder
alphabetically, and a folder exists only while a page lives in it. Users want
the tree to be theirs: put "Inbox" above "Archive", keep a project folder
ready before writing its first page, and move whole folders around. Today the
only way to shape the tree is to rename things until the alphabet agrees.

## What Changes

- **Custom order:** every row (page or folder) can sit at any position among
  its siblings. The order is stored in one vault file, `.cerbo/tree.toml`, as
  an ordered list per folder. Rows without a stored position follow the
  stored ones in today's default order (folders first, then titles A to Z).
- **Empty folders:** a folder can be created on its own and stays in the tree
  until it is deleted, even with no pages inside. Such folders are listed in
  the same file. A folder that only exists because of page paths still
  disappears when its last page leaves, as today.
- **Drag to reorder and move:** dragging a row shows where it will land: a
  line above or below a row places it before or after that row (moving it
  into that row's folder if needed); a highlighted folder row puts it inside.
  Folder rows become draggable; dropping a folder moves it with everything
  under it.
- **Keyboard and menu:** `Alt+ArrowUp`/`Alt+ArrowDown` (and `Alt+k`/`Alt+j`)
  move the focused row one place among its siblings. The row context menu
  gains "Move up" and "Move down"; the folder menu gains "New folder here" and
  "Delete folder" (empty folders only).
- **Add menu:** right-clicking empty space in the sidebar opens an "Add" menu
  with "Page" and "Folder", both at the root.
- **Core, Tauri and CLI:** a `tree` module in `cerbo-core` reads and writes
  `.cerbo/tree.toml`; `folder_rename` keeps it in step. New CLI commands
  `cerbo folder create`, `cerbo folder delete` and `cerbo folder order`, and
  `cerbo folder list` to print the ordered tree. Man page updated.

Not **BREAKING**: a vault without `tree.toml` looks exactly as today, and
older builds ignore the file.

## Capabilities

### New Capabilities

- `vault-tree-layout`: the `.cerbo/tree.toml` file: format, order rules,
  pinned (empty) folders, how page moves, folder renames and deletes keep it
  consistent, and the CLI entry points.
- `desktop-tree-editing`: reordering rows by drag, keyboard and menu, folder
  drag, the Add menu, creating and deleting folders in the desktop app.

### Modified Capabilities

- `desktop-page-tree` (added by `add-virtual-path-tree`): rows render in the
  stored order and pinned folders render even when empty.
- `cli-tooling`: new `cerbo folder` subcommands.

## Non-goals

- Ordering in `cerbo page list` or in the `cerbo symlink` tree. Both keep
  their current order.
- Storing order in RDF (`meta.ttl`) or querying it with SPARQL.
- Multi-select drag, and drag between windows or vaults.
- Live refresh of the desktop tree when another process edits `tree.toml`.
  The desktop picks it up on its next page reload, like `virtualPath` edits
  made by the CLI today.

## Impact

- **Core:** new `core/src/tree.rs`; `core/src/page.rs` (`folder_rename`
  rewrites the layout file); `core/src/lib.rs`.
- **Tauri:** `src-tauri/src/page.rs` new commands, registered in `lib.rs`.
- **CLI:** `cli/src/main.rs`, `cli/man/cerbo.md`, `docs/cli.md`,
  `.agents/skills/cerbo-cli/SKILL.md`, `cli/tests/cli_integration.rs`.
- **Frontend:** `src/lib/page-tree.ts` (order and pinned folders),
  `src/lib/tree-drag.ts` (drop zone maths), `src/lib/PageList.svelte`,
  new `NewFolderDialog.svelte`, `stores.svelte.ts`, `KeyboardHelp.svelte`,
  `docs/desktop.md`.
- **Vault data:** one new optional file per vault, `.cerbo/tree.toml`.
