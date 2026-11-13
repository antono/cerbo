# Proposal

## Why

Cerbo records knowledge as RDF, but nothing can query it as a whole, and notes cannot be found by meaning. Backlinks are cached in a `backrefs.ttl` file inside each linked object. That file is derived data stored as if it were source data, so one save rewrites other objects' files and causes sync conflicts on hub pages. A queryable index makes this cache unnecessary.

Depends on `normalize-rdf-vocabulary`, which lands first.

## What Changes

- Add a **derived semantic index** per vault and machine in `$XDG_CACHE_HOME/cerbo/index/`, never inside the vault. It is built automatically when missing and catches up with files changed by git, sync or external editors.
- The index holds each object's `meta.ttl`, `annotations.ttl` and its outgoing links from `page.md` as `schema:citation`.
- **BREAKING (on-disk):** `backrefs.ttl` is removed. Backlinks become an index query. Saving a page writes only that page's files. `cerbo index` deletes leftover `backrefs.ttl` files.
- The desktop backlinks panel and `attachment_list` stop reading `backrefs.ttl`. New `cerbo page backlinks <uuid>`.
- Add `cerbo sparql '<query>'` (read-only SPARQL).
- Compute **local text embeddings**. The model is pinned to one revision with SHA-256 checksums, downloaded on first use or loaded from `CERBO_MODEL_DIR`. Page text never leaves the machine.
- Add `cerbo search "<text>"` and `cerbo similar <uuid>` with graph filters.
- Derive implicit `cerbo:similarTo` links inside the index only.
- `cerbo index` gains `--embeddings` and rebuilds the graph index.

## Capabilities

### New Capabilities

- `semantic-graph-index`: the per-vault index (location, contents, sync, catch-up, rebuild, locking).
- `graph-query`: read-only SPARQL through `cerbo sparql`.
- `semantic-search`: embeddings, pinned model, `cerbo search`, `cerbo similar`, implicit similarity links.

### Modified Capabilities

- `backlinks`: computed from the index. `backrefs.ttl` is gone. New CLI listing.
- `desktop-backlinks-panel`: `backlinks_get` queries the index.
- `page-crud`: saving and deleting write no other object's files.
- `uuid-object-storage`: objects have no `backrefs.ttl`.
- `page-metadata-index`: `cerbo index` rebuilds the graph and embeddings, writes no backrefs and removes legacy ones.

## Non-goals

- No SPARQL UPDATE. Vault files stay the only place where data is written.
- No new desktop UI apart from the backlinks panel states. A "similar notes" panel follows separately.
- No remote embeddings.
- No keyword/BM25 search.
- No cross-vault queries.
- No inference or SHACL.
- `cerbo:similarTo` is never written into vault files.

## Impact

- **Code:**
  - New core modules for the graph index, vector index and embedder.
  - `object.rs` drops `update_backrefs` in favour of index hooks; `links.rs` loses the backrefs file API.
  - `metadata_index.rs` stops writing backrefs.
  - `src-tauri/src/index.rs` and new CLI subcommands.
- **Dependencies:**
  - `oxigraph` 0.5.x with RocksDB (pins `oxrdf =0.3.5`/`oxttl =0.2.5`, matching core).
  - `fastembed` 7.x with ONNX Runtime loaded dynamically.
  - An HTTP client and `sha2`.
  - Nix provides RocksDB and ONNX Runtime.
- **Disk:** no new files in vaults. Index and model live in the XDG cache.
- **Behaviour:** the index is now required for backlinks. If it is busy, backlinks are briefly unavailable, but saving never fails.
- **Docs:** man page, `docs/storage.md`, `README.md`.
