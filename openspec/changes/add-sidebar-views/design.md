# Design (DRAFT)

## Context

- Tree today = pure function of `app.pages` (each page's `virtualPath`) plus
  `.cerbo/tree.toml` (order, pinned folders). `core/src/tree.rs` builds the
  ordered `TreeNode`s; the frontend flattens them for keyboard handling.
- `ObjectType` is `Product | Source | Attachment | Ontology`, each mapped to a
  schema.org class in `meta.ttl`.
- Placement (`virtualPath`) is a property of the page. A second tree cannot
  reuse it, so a view must keep its own structure.

## Decisions

### D1. A view is an object, its tree is a file next to `meta.ttl`

`View` joins `ObjectType` (class: `schema:ItemList`).
`meta.ttl` carries title and type, so views are found, trashed and indexed
like every object. The tree lives in `view.toml` in the object directory,
not in RDF.

Why a file, not triples: an ordered nested tree is awkward in RDF (lists,
blank nodes) and the existing `tree.toml` already proves TOML is a good
fit for humans and agents to read and edit. Cost: the tree is not
SPARQL-queryable. Mitigation: `meta.ttl` also records `schema:hasPart` links
to the referenced pages, so "which views contain this page" is a SPARQL query
and the backlinks panel can show it.

Alternative rejected: one `views.toml` for all views. It would not be an
object (no trash, no sync granularity, no per-view history).

### D2. `view.toml` format

```toml
# Berlin trip
[[node]]
kind = "folder"
name = "Flights"
  [[node.children]]
  kind = "page"
  uuid = "137-..."          # page reference

[[node]]
kind = "query"
name = "Open questions"
sparql = "SELECT ?page WHERE { ?page schema:keywords 'berlin' }"
```

Kinds: `folder` (label + children), `page` (UUID), `query` (SPARQL returning
`?page`). Same page may appear many times, in one or several views.
`deny_unknown_fields`, and a file that does not parse leaves the view
unrendered with an error naming it, never partial writes (same rule as
`tree.toml`).

### D3. Default view is built in

"All notes" is computed, not stored. It guarantees every page is reachable and
gives the agent a safe baseline. Its edits (drag, reorder) keep writing
`virtualPath` / `tree.toml` exactly as today. `View` objects are excluded
from `page_list`, so they never appear as pages.

### D4. Dangling references

A `page` node whose UUID no longer exists (deleted/trashed) renders as a
muted "missing page" row with "Remove from view". Nothing is rewritten
automatically; core does not scan views on page delete. Query nodes just
return fewer rows.

### D5. Query nodes

Run through the existing SPARQL engine (`graph-query`), read-only, result must
bind `?page` to page IRIs; other bindings are ignored. Evaluated on view
open and on page-list reload, not continuously. A failing query shows an
inline error on that node only.

### D6. Core and transport API

```rust
// core/src/view.rs
pub fn view_list(ctx) -> Result<Vec<ViewMeta>, String>
pub fn view_get(ctx, uuid) -> Result<ViewTree, String>      // resolved tree
pub fn view_create(ctx, title, tree: ViewSpec) -> Result<String, String>
pub fn view_update(ctx, uuid, tree: ViewSpec) -> Result<(), String>
pub fn view_delete(ctx, uuid) -> Result<(), String>        // to trash
```

One core function per operation shared by CLI, Tauri and the daemon RPC, same
error strings (as in `add-virtual-path-tree`). `view_update` replaces the
whole tree atomically; fine-grained ops (move node) are UI-side and call
`view_update`.

### D7. Agent access

MCP tools mirror D6. The agent sends a `ViewSpec` (JSON of the same shape as
`view.toml`); core validates every UUID and SPARQL syntactically before write.
Permission: `read` hides write tools; `approve` shows an approval card with
the view title and node count; `auto` runs. An agent cannot change the
default view or any page's `virtualPath` through these tools.

### D8. Frontend

`app.activeViewId: string | null` (`null` = All notes), stored per vault in
`ui.toml` (same place as collapsed folders, pruned if the view disappears).
`page-tree.ts` gets a second builder `buildViewTree(view, pages)` returning
the same `TreeNode` shape, so flattening, keyboard and rendering code stay
shared. Drag-and-drop in a non-default view edits `view.toml` only; a
`page` row dragged out of the sidebar into another view is "add reference",
not "move".

## Open questions (need a decision)

1. ~~**Membership model**~~ **Decided (2026-11-20): explicit tree + query
   nodes** (D2). Query-only views are out of scope for v1.
2. ~~**Same page in several places of one view**~~ **Decided: allowed**
   (D2).
3. ~~**Class IRI** for `View`~~ **Decided: `schema:ItemList`** (D1).
4. ~~**Do views sync/export** with `cerbo symlink`?~~ **Decided: no.** The
   symlink tree stays on `virtualPath`.
5. ~~**Switcher placement**~~ **Decided: dropdown above the tree** (D8).

No open questions remain; next step is writing `specs/`.

## Risks

- A page deleted while it is in many views leaves many dangling rows (D4);
  acceptable for v1.
- SPARQL nodes can be slow on big vaults; mitigated by evaluating once per
  open, with a result cap (e.g. 137 rows per node, "show more").
