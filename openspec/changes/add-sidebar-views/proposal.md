# Proposal (DRAFT)

## Why

The sidebar shows one tree: the pages arranged by `virtualPath`, ordered by
`.cerbo/tree.toml`. That tree is the vault's default structure, but a task
often needs another one: "everything for the Berlin trip", "open
questions this week", "reading list by topic". Today the only way to get such a
shape is to move pages in the one shared tree, which breaks the default
structure for everything else.

A **view** is an alternative tree over the same pages. Pages are never copied
or moved by a view; a view only says which pages it shows and how they are
grouped and ordered. A connected agent can create a view on request, and the
user switches between views in the sidebar.

## What Changes

- **New object type `View`** (like `Product`, `Source`, `Ontology`): lives in
  `.cerbo/objects/<uuid>/`, has a `meta.ttl` (title, type) and a `view.toml`
  with its tree. It is a normal object: synced, indexed, trashable, queryable.
- **Built-in default view "All notes"**: not an object, cannot be deleted or
  edited. It is today's tree (`virtualPath` + `tree.toml`) and always shows
  every page, so no page can be lost from sight.
- **View content**: an ordered tree of nodes. A node is a *folder* (label
  only), a *page reference* (UUID), or a *query node* (a SPARQL SELECT that
  returns page UUIDs, rendered as a live list inside the folder).
- **Sidebar switcher**: a selector above the tree (default: "All notes");
  the choice is remembered per vault. Drag/reorder in a view edits that view
  only; `virtualPath` is untouched.
- **Agent tools** (`cerbo mcp`, CLI): `view_list`, `view_get`, `view_create`,
  `view_update`, `view_delete`. These are write tools: hidden at `read`
  permission, approval card at `approve`.
- **CLI**: `cerbo view list|show|create|set|delete`; man page updated.

## Capabilities

### New Capabilities
- `vault-views`: the `View` object type, `view.toml` format, default view,
  node kinds, validation, how deleted/missing pages are handled.
- `desktop-view-switcher`: the selector, per-vault remembered choice, editing
  a view in the tree.
- `agent-view-tools`: MCP tools and permission behaviour.

### Modified Capabilities
- `desktop-page-tree`: the tree is rendered from "the active view" instead of
  always from `app.pages` + layout.
- `ontology-management` / `page-metadata-index`: `View` is added to the object
  classes; it must not appear as a page in "All notes".
- `cli-tooling`, `cerbo-mcp`: new commands and tools.

## Non-goals

- Several windows showing different views at the same time.
- Sharing or exporting views outside the vault.
- Views that change page content or `virtualPath`.
- Live refresh when another process edits a view (same limitation as
  `tree.toml` today).

## Impact

- Core: `core/src/object.rs` (new variant), new `core/src/view.rs`,
  `core/src/page.rs` (exclude `View` from page lists), `core/src/rpc`.
- Tauri: view commands in `src-tauri`.
- CLI/MCP: `cli/src/main.rs`, `cli/src/mcp.rs`, `cli/man/cerbo.md`,
  `.agents/skills/cerbo-cli/SKILL.md`.
- Frontend: `src/lib/page-tree.ts` (build tree from a view),
  `PageList.svelte`, `stores.svelte.ts`, new `ViewSwitcher.svelte`.
- Vault data: new object directories of type `View`.

## Depends on

`add-virtual-path-tree` and `add-sidebar-ordering` archived first (this change
modifies `desktop-page-tree`). Tools part benefits from
`add-agent-host-scope-and-mcp-skill` (skill text must mention views).
