# Tasks

Prerequisites: `normalize-rdf-vocabulary` and `add-oxigraph-semantic-index` are implemented, including its backlinks panel task 3.6.

## 1. Tauri commands

- [ ] 1.1 Add async commands `similar_get(uuid, limit)`, `embeddings_refresh()` (with `semantic://progress` events), `prefix_map_get()` and `sparql_query(query)` in `src-tauri/src`, run core work in `spawn_blocking`, register them in `lib.rs`; verify `nix build .#cerbo-desktop` succeeds and a Rust unit test covers argument and result mapping where the command layer adds logic
- [ ] 1.2 Add `last_sparql_query` to `UiSettings` with serde default; verify the core round-trip test reads an old `ui.toml` without the field

## 2. Similar notes panel

- [ ] 2.1 Split `RightSidebarPanel.svelte` into sections and add `SimilarNotesSection.svelte` with list, scores, empty, disabled ("Enable"), progress, unavailable-model and busy states; add store state and loaders in `stores.svelte.ts` with the UUID guard from design D2; verify vitest tests for the store logic (state transitions, stale response dropped)
- [ ] 2.2 Verify manually on `nix build .#cerbo-desktop` with `test-vault`: enable embeddings from the panel, navigate between pages, type during a refresh

## 3. Mention rendering

- [ ] 3.1 Implement `src/lib/mention-plugin.ts` and register it in `PageEditor.svelte`; load the prefix map on vault open; add `.mention` and `.mention.unresolved` styles using existing design tokens in both themes; verify vitest tests for resolved, unresolved, adjacent link, task list and wikilink cases, and that braces never appear in the output

## 4. SPARQL console

- [ ] 4.1 Implement `SparqlConsole.svelte` (query field, Run, `Ctrl+Enter`, results table/ASK/Turtle, row count, truncation notice, error with position, IRI → title link, prefixed display, last query restore); verify vitest tests for result mapping and IRI compaction
- [ ] 4.2 Add the `Ctrl+Shift+E` shortcut in `+layout.svelte` (suppressed while a confirmation modal is open) and the entry in `KeyboardHelp.svelte`; verify the existing hotkey tests and a new one for the shortcut

## 5. Checks

- [ ] 5.1 Run `bun run check`, `bunx vitest run` (inside `nix develop`) and `nix build .#cerbo-desktop`, and `openspec validate add-semantic-desktop-ui --strict`; verify all pass
- [ ] 5.2 Manual pass on the built app in light and dark themes: similar notes, mention tooltips, console navigation; verify no console errors
