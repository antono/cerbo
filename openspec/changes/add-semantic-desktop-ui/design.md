# Design

## Context

- The frontend is SvelteKit with Svelte 5 runes. App state is in `src/lib/stores.svelte.ts`.
- The preview is rendered by Carta (`carta-md`) with remark/rehype plugins. `src/lib/wikilink-plugin.ts` is the model for a custom inline syntax.
- `RightSidebarPanel.svelte` holds only Backlinks today. `add-oxigraph-semantic-index` gives it index-backed loading and busy states.
- Tauri commands live in `src-tauri/src` and are registered in `lib.rs`. They call `cerbo-core` synchronously.
- Shortcuts are handled in `src/routes/+layout.svelte` with `isModKey` from `hotkeys.ts`. `Ctrl+Shift+E` is unused.
- UI settings persist in `ui.toml` through `core/src/ui_settings.rs`.

## Goals / Non-Goals

**Goals:**
- No semantic work on the UI thread. The editor never waits for the index or the model.
- One code path per operation, shared with the CLI through core.

**Non-Goals:**
- New core algorithms. This change only exposes what the prerequisites build.

## Decisions

### D1. Long operations as async Tauri commands with progress events

`similar_get`, `embeddings_refresh` and `sparql_query` are `async` commands that run core work in `tauri::async_runtime::spawn_blocking`. `embeddings_refresh` emits `semantic://progress` events (`{ phase: "download" | "embed" | "similarity", done, total }`), which the section renders.

- **Alternative:** synchronous commands, as used by existing commands. These would freeze the window during a model download.

### D2. Stale-response guard

Each similar-notes request carries the UUID it was made for. The section keeps only a response whose UUID equals the currently open page. This covers "Navigating away during loading" without cancellation plumbing.

### D3. Mention plugin with a per-vault prefix map

A Carta plugin, `mention-plugin.ts`, parses `[Text]{prefix:Type}` in mdast:
- It replaces each match with a `span.mention` (or `span.mention.unresolved`) and puts the type and IRI in `title` and `data-*` attributes.
- It resolves prefixes from `prefix_map_get`, which returns the vault's resolved map, built-ins included. The map is loaded on vault open and cached in the store, so rendering stays synchronous.
- It runs before link handling, so `[Berlin]{…}` is never parsed as a link reference.

### D4. Console results as SPARQL JSON

`sparql_query` returns the core result as SPARQL Results JSON (SELECT/ASK) or a Turtle string (CONSTRUCT/DESCRIBE), truncated to 1000 rows by core, with a `truncated` flag. The frontend:
- maps `cerbo://objects/<uuid>` cells to titles with one extra lookup per unique UUID, through the existing page list in the store;
- compacts other IRIs with the prefix map from D3.

### D5. Last query in `UiSettings`

`UiSettings` gains `last_sparql_query: Option<String>`. It is global per app rather than per vault, which matches the spec ("last query run in this app") and needs no new storage.

## Risks / Trade-offs

- [A model download started from the panel takes minutes] → Progress events and a non-blocking section. The download can also be done ahead of time with `cerbo index --embeddings`.
- [Mention regex collides with other bracket syntax (task lists, links)] → The plugin matches only `]{prefix:Name}` with an identifier-shaped prefix. Unit tests cover links, task lists and wikilinks next to mentions.
- [Many unique UUIDs in a large SELECT] → Titles come from the in-memory page list. Unknown UUIDs show the raw IRI.
