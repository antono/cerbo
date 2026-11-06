# Tasks

Archive `add-virtual-path-tree` before this change: the `desktop-page-tree`
delta here modifies a spec that change adds.

## 1. Core: layout file

- [x] 1.1 Add `core/src/tree.rs` with `TreeLayout { pinned, order }`, read (missing file = empty, parse error names the file) and atomic sorted write; verify unit tests for missing file, round trip, sorted and deduplicated output, and a broken file left unchanged after a failed write
- [x] 1.2 Implement the D2 display order as `folder_tree(ctx) -> Vec<TreeNode>` from `page_list` and the layout, including pinned folders and their ancestors; verify unit tests for listed-first, unlisted default order, stale and duplicate entries skipped, pinned empty folder, flat vault unchanged
- [x] 1.3 Implement `folder_create` (validate, refuse existing derived or pinned folder, pin) and `folder_delete` (refuse with page count, drop pins, order keys and the parent entry); verify unit tests for each scenario in `vault-tree-layout`
- [x] 1.4 Implement `folder_set_order(parent, entries)` with entry validation and pruning; verify unit tests for reorder, pruning unknown entries and duplicates, rejecting a malformed entry without writing
- [x] 1.5 Make `page::folder_rename` call `tree::rename_prefix` after the page writes, and rename a pinned-only folder; verify unit tests for position kept, nested order and pins carried, move to another parent removes the old entry, pinned empty folder renamed

## 2. CLI

- [x] 2.1 Add `cerbo folder list|create|delete|order` with `--json`; verify `cli_integration.rs` tests for every `cli-tooling` scenario in this change
- [x] 2.2 Document the four commands and `tree.toml` in `cli/man/cerbo.md`, `docs/cli.md` and the `cerbo-cli` skill; verify `nix build .#cerbo` renders the man page with them

## 3. Tauri commands

- [x] 3.1 Add `tree_layout`, `folder_create`, `folder_delete`, `folder_set_order` in `src-tauri/src/page.rs` via `off_main` and register them; verify `cargo build -p cerbo-desktop` (or `nix build .#cerbo-desktop`) succeeds

## 4. Frontend: model

- [x] 4.1 Extend `buildPageTree(pages, layout)` in `page-tree.ts` with the D2 order and pinned folders; mirror the Rust test cases in `page-tree.test.ts`
- [x] 4.2 Add `entryKey(row)`, `siblingsOf(tree, parent)` and `placement(tree, row, parent, index)` returning the move and the new order (D6); verify vitest cases for same-folder reorder, cross-folder page move, folder move, no-op when the row stays put, refusal of a folder into its own subtree
- [x] 4.3 Add `src/lib/tree-drag.ts` with the D7 zone function `dropZone(row, rect, y)` and the drag threshold; verify vitest cases for every row of the D7 table
- [x] 4.4 Load the layout with the page list in `stores.svelte.ts`; add `placeRow`, `createFolder`, `deleteFolder`, `moveRowBy(row, ±1)` running inside `vaultWrite` and reloading once; verify `bun run check`

## 5. Frontend: interaction

- [x] 5.1 Make folder rows draggable and switch the drop logic to `placement` with before/after lines and inside highlight; verify e2e cases for reorder, move-and-place, drop inside, folder drag, no indicator over own subtree
- [x] 5.2 Add `Alt+ArrowUp/Down` and `Alt+k/j` to the sidebar handler and the help screen; verify an e2e case moves a page up and keeps focus
- [x] 5.3 Extend the row menu with "Move up"/"Move down" and the folder entries, and add the Add menu on empty sidebar space; verify e2e cases for menu move, delete empty folder, no delete on a folder with pages, Add > New folder
- [x] 5.4 Add `NewFolderDialog.svelte` with `VirtualPathInput` (`#new-folder-path`), prefill for "New folder here", error display; verify e2e cases for nested create and existing-folder error

## 6. Docs and checks

- [x] 6.1 Update `docs/desktop.md` with ordering, folder drag, the Add menu and the keys
- [x] 6.2 Run `cargo test --workspace`, `cargo clippy`, `bun run test`, `bun run check`, `openspec validate --strict` and the desktop e2e suite on `nix build .#cerbo-desktop`
- [ ] 6.3 Manual pass on GNOME Wayland with `test-vault`: drag reorder, folder drag, Add menu, keyboard moves, both themes
