# Proposal

## Why

`normalize-rdf-vocabulary` and `add-oxigraph-semantic-index` give Cerbo a typed graph, meaning-based similarity and SPARQL, but only the CLI can use them. The desktop app should show related notes while writing, make annotations visible as typed mentions, and let users explore the graph directly.

Depends on both changes above. The backlinks panel itself is updated inside `add-oxigraph-semantic-index`.

## What Changes

- **Similar notes panel:** a second section in the right sidebar, below Backlinks, listing the notes closest in meaning to the open page, with scores. It explains how to enable embeddings when they are off, shows download and refresh progress, and never blocks editing.
- **Mention rendering:** in preview, `[Berlin]{schema:Place}` renders as styled text with a tooltip showing the type (`schema:Place` and its full IRI). Unresolvable prefixes are styled as warnings. The preview never renders them as links.
- **SPARQL console:** a dialog opened with `Ctrl+Shift+E` (`Cmd+Shift+E` on Mac) to run read-only SPARQL against the current vault, with predeclared prefixes. SELECT shows a table in which object IRIs open their page, ASK shows true/false, and CONSTRUCT shows Turtle. Errors show their position. The last query is remembered.
- New Tauri commands: `similar_get`, `embeddings_refresh`, `prefix_map_get` and `sparql_query`. Each one is a thin wrapper over `cerbo-core`.
- The shortcuts help lists the new shortcut.

## Capabilities

### New Capabilities

- `desktop-similar-notes-panel`: the similar-notes sidebar section and its states.
- `desktop-sparql-console`: the SPARQL console dialog.

### Modified Capabilities

- `semantic-annotations`: annotation rendering shows the resolved type and flags unresolved prefixes.
- `keyboard-shortcuts`: the console shortcut.

## Non-goals

- Editing the graph from the console (no SPARQL UPDATE).
- Graph visualisation (a node-link view).
- Showing mentions in the raw editor mode. Preview only.
- Semantic search inside the existing `Ctrl+P` page search. That is a possible follow-up.

## Impact

- **Frontend:**
  - `src/lib/RightSidebarPanel.svelte` (sections) and a new `SimilarNotesSection.svelte`.
  - A mention Carta plugin next to `wikilink-plugin.ts`.
  - A new `SparqlConsole.svelte`.
  - `src/routes/+layout.svelte` (shortcut), `KeyboardHelp.svelte` and `stores.svelte.ts`.
- **Tauri:** `src-tauri/src` commands registered in `lib.rs`. Long operations run off the UI thread and emit progress events.
- **Core:** no new logic beyond the public functions the two prerequisite changes add, plus `UiSettings.last_sparql_query`.
