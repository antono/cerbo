# Tasks

Prerequisite: `normalize-rdf-vocabulary` is implemented and archived.

## 1. Dependencies and feasibility checks

- [x] 1.1 Add `oxigraph = "0.5"` (default `rocksdb` feature) as a normal dependency of `core`; verify `cargo tree -p cerbo-core -i oxrdf` shows a single `oxrdf 0.3.5` and `cargo check -p cerbo-core` passes inside `nix develop`
- [x] 1.2 Add `fastembed` 7.x (default features off, `ort-load-dynamic`), an HTTP client and `sha2` behind a new `embeddings` cargo feature; record the SHA-256 of the five pinned model files at commit `614241f622f53c4eeff9890bdc4f31cfecc418b3` as constants; confirm mean pooling from `1_Pooling/config.json`; add `onnxruntime` to the dev shell with `ORT_DYLIB_PATH`; verify a scratch test embeds "поездка в Берлин" and "trip to Berlin" with cosine > 0.8
- [x] 1.3 Wrap the `cerbo` and `cerbo-desktop` Nix packages with `ORT_DYLIB_PATH` and add Clang/RocksDB build inputs as needed; verify `nix build .#cerbo .#cerbo-desktop` succeeds
- [x] 1.4 Measure Store open+close plus the fingerprint catch-up scan (no changes) on `test-vault` and on a synthetic vault of 137 000 objects; verify the numbers are recorded in design.md under D3/D12 and, if either exceeds 200 ms, raise it with the user before continuing
- [x] 1.5 Confirm `std::fs::File::lock` is available on the pinned toolchain (or pick `fs4`); verify by a unit test that a second lock attempt from another process is refused

## 2. Graph index in core

- [x] 2.1 Create `core/src/semantic/mod.rs` with the `GraphIndex` trait, the index directory resolver (`$XDG_CACHE_HOME/cerbo/index/<sha256(canonical vault path)[..16]>/`) and `IndexState` (`state.json`: `format_version`, `generation`, `stale`, `vault_path`, per-page content hashes); verify unit tests for the path key, a key collision detected through `vault_path`, and the load/save round trip with the `fsio` durable write
- [x] 2.2 Implement the advisory `LOCK` with retry and backoff up to 5 s and an `IndexBusy` error; verify a test holding the lock in another thread gets `IndexBusy` after the timeout
- [x] 2.3 Implement `OxigraphIndex::upsert_object` / `remove_object`: one named graph `cerbo://graphs/<uuid>` per object with `meta.ttl`, `annotations.ttl` and one `schema:citation` per distinct `cerbo://` link in `page.md` (broken targets included), via `clear_graph` plus load in one transaction; verify tests for insert, replace (removed annotation and removed link disappear), remove, and a broken link
- [x] 2.4 Implement the fingerprint table and the catch-up step from design D12 (hybrid): full scan with `stat` on 8 threads (upsert changed or new, remove vanished, full re-upsert when `stale`), plus an incremental `catch_up_paths(changed_uuids)` for the desktop; wire the desktop to run the full scan on vault open and the incremental path from the existing `notify` watcher (debounced), with a full scan after watcher errors; verify tests where a file is added, edited and deleted behind Cerbo's back and the next query sees each change, and a test for the incremental path
- [x] 2.5 Implement automatic build when the index is missing or its format version differs, with a progress callback; verify tests for "Query before first index", "Index from an older Cerbo version" and "Vault moved to another path"
- [x] 2.6 Implement the full rebuild into `graph-<gen+1>/` with an atomic `state.json` switch and old-generation cleanup; verify a test that aborts mid-rebuild still queries the previous generation
- [x] 2.7 Implement the query entry point with predeclared prefixes (`cerbo`, the built-in well-known prefixes, and the vault's `ontology-map.json`, in that override order), union default graph applied only when the query has no FROM/FROM NAMED, and rejection of updates; verify tests for each scenario in `specs/graph-query`

## 3. Backlinks from the index (remove `backrefs.ttl`)

- [x] 3.1 Add `backlinks(vault_ctx, uuid) -> Vec<BacklinkEntry>` in core (SPARQL from design D11, distinct, ordered by title then UUID); verify the `backlinks` scenarios "Page linked from two pages", "Page linked from nowhere" and "Linking page moved to trash"
- [x] 3.2 Replace the index hooks: after successful writes in `object_create*`, `object_write` and `object_delete`, upsert or remove only that object's graph; reduce `update_backrefs` to broken-link detection with no writes; verify the integration test where saving a link makes the backlink visible without reindexing and no file of the target changes (compare mtimes)
- [x] 3.3 Make index failures log a warning and set `stale: true` without failing the write; verify a test that holds `LOCK` during `object_write` and asserts the files are written, the state is stale, and the next query repairs it
- [x] 3.4 Rewrite `attachment_list` to parse `page.md` links and keep targets whose type is Attachment; verify existing attachment tests still pass
- [x] 3.5 Delete the `backrefs.ttl` API from `core/src/links.rs` (`backrefs_read*`, `backrefs_add*`, `backrefs_remove*`, `backrefs_set_vault`) and its tests; verify `rg backrefs core cli src-tauri` finds only the legacy-file cleanup in task 4.1
- [ ] 3.6 Switch the Tauri `backlinks_get` to `cerbo_core::semantic::backlinks`, return a distinct "index busy" error, and make the panel in `RightSidebarPanel.svelte` show a non-blocking message and a loading state while the index builds; verify the `desktop-backlinks-panel` scenarios manually on a build from `nix build .#cerbo-desktop`
- [x] 3.7 Add `cerbo page backlinks <uuid> [--json] [--vault]`; verify CLI tests for "Backlinks as JSON" and "Unknown object"

## 4. `cerbo index` and `cerbo sparql`

- [x] 4.1 Change `metadata_index` so `cerbo index` stops writing backrefs, regenerates annotations, deletes leftover `backrefs.ttl` of live objects (counted), and updates the graph for all scopes; add `triples` and `legacy_backrefs_removed` to the summary (`--json` included); verify the `specs/page-metadata-index` delta scenarios as CLI tests, including 137 legacy files removed
- [x] 4.2 Add `cerbo sparql <query|->` with `--file`, `--vault` and `--json` (SPARQL Results JSON / JSON-LD); table/`true|false`/Turtle output by default; verify CLI tests for every `specs/graph-query` scenario
- [x] 4.3 Document `cerbo sparql`, `cerbo page backlinks`, the new `cerbo index` behaviour, the cache location, the removal of `backrefs.ttl` and the "upgrade all devices" note in `cli/man/cerbo.md`; update `docs/storage.md` and `README.md`; verify the man page builds (`nix build .#cerbo`) and `man ./result/share/man/man1/cerbo.1` shows the new sections

## 5. Embeddings and vector search

- [x] 5.1 Implement the `Embedder` trait with the pinned `multilingual-e5-small` loader: download into `$XDG_CACHE_HOME/cerbo/models/<commit>/` via temp file, SHA-256 check, rename; `CERBO_MODEL_DIR` with verification and no download; load with `UserDefinedEmbeddingModel` and mean pooling; `passage:`/`query:` prefixes, 512-token truncation, progress on stderr; verify unit tests for checksum mismatch (file deleted, error names it), `CERBO_MODEL_DIR` with wrong files (no download attempted), and a mock embedder, plus one ignored-by-default test against the real model
- [x] 5.2 Implement the `VectorIndex` trait with the flat `f32` store (`vectors-<gen>.bin` plus UUID table, normalised, dot-product top-k, delete); verify unit tests for exact ordering, delete and reload
- [x] 5.3 Implement the lazy refresh: hash title+`page.md`, re-embed changed pages, drop removed ones; verify a test that two `refresh` runs with one edited page embed exactly one page
- [x] 5.4 Implement similarity links (top 5, score ≥ 0.80) in `cerbo://graphs/similarity` as `cerbo:similarTo` plus a `cerbo:Similarity` node with the score; verify a SPARQL test finds the pair and a test asserts no vault file contains `similarTo`
- [x] 5.5 Implement the graph-filtered kNN (`VALUES` plus `--type`/`--annotated`, widening the candidate set until `--limit` survivors or the vault is exhausted); verify the "filters do not shrink the result count" scenario with 137 pages and 20 `schema:Place` pages using the mock embedder

## 6. `cerbo search`, `cerbo similar`, `cerbo index --embeddings`

- [x] 6.1 Add `cerbo index --embeddings` (refresh plus similarity, `embedded` in the summary and JSON); verify the `specs/page-metadata-index` "Refresh embeddings on request" scenario
- [x] 6.2 Add `cerbo search <text>` and `cerbo similar <uuid>` with `--limit` (default 10), `--type`, `--annotated`, `--json` and `--vault`, the "run `cerbo index --embeddings`" error, and "Object not found"; verify CLI tests for every `specs/semantic-search` scenario using the mock embedder
- [x] 6.3 Document `cerbo search`, `cerbo similar`, `--embeddings`, the pinned model, `CERBO_MODEL_DIR` and offline behaviour in `cli/man/cerbo.md`; verify the man page builds and lists the commands

## 7. Integration checks

- [x] 7.1 On `test-vault` with the real model: run `cerbo index --embeddings`, then `cerbo search "поездка в Берлин"` and `cerbo similar <uuid>`; tune the 0.80 threshold if results are empty or noisy and record the value in design.md D7
- [ ] 7.2 Run the CLI while the desktop app edits the same vault (`nix build .#cerbo-desktop`); verify no crash, the CLI waits or reports "index busy", `cerbo sparql` and `cerbo page backlinks` reflect the app's edits, and the panel shows the CLI's edits
- [x] 7.3 Simulate sync: copy a page directory that links to an existing page into `test-vault/.cerbo/objects/` with `cp -rf`; verify the target's backlinks list the copied page without running `cerbo index`
- [x] 7.4 Remove all `backrefs.ttl` from `test-vault` with `cerbo index` and commit the result with the implementation; verify `git status` shows only those deletions plus intended changes
- [x] 7.5 Run `cargo test -p cerbo-core --all-features -- --test-threads=1`, `cargo test -p cerbo`, `nix build .#cerbo .#cerbo-desktop` and `openspec validate add-oxigraph-semantic-index --strict`; verify all pass

## Workflow follow-up

- Archive the change after review with `/opsx-archive`. At archive time, update the `backlinks` spec Purpose and remove the now-vacuous backreference clauses from `durable-writes` and `object-trash`.
- Plan a separate change for the desktop "Similar notes" panel and Tauri commands.
