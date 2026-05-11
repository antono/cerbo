## 1. Core Library Refactoring

- [x] 1.1 Make `extract_cerbo_links()` public in `core/src/links.rs`
- [x] 1.2 Make `extract_annotations()` public in `core/src/annotations.rs`
- [x] 1.3 Add `backrefs_clear()` function in `core/src/links.rs` to reset a vault's backrefs
- [x] 1.4 Add public API in `core/src/lib.rs` for indexing operations

## 2. Vault/Page Discovery

- [x] 2.1 Add `list_all_vaults()` function in `core/src/vault.rs` to enumerate all vaults
- [x] 2.2 Add `list_pages_in_vault(vault_uuid)` function to enumerate pages in a vault
- [x] 2.3 Add `get_page_path(page_uuid)` helper to resolve page object directory

## 3. Indexing Logic

- [x] 3.1 Create `core/src/metadata_index.rs` module for indexing operations
- [x] 3.2 Implement `index_vault(vault_ctx)` - two-pass within vault scope
- [x] 3.3 Implement `index_page(vault_ctx, page_uuid)` - incremental single-page indexing
- [x] 3.4 Add error handling for corrupted/missing page.md files (log and continue)
- [ ] 3.5 Add warning logging for broken link references (nonexistent UUIDs)

## 4. Progress Feedback

- [x] 4.1 Create progress tracking struct (IndexStats: pages_processed, links_found, annotations_found, errors)
- [ ] 4.2 Add periodic progress output to stderr ("Processing N/M pages...")
- [x] 4.3 Add summary statistics on completion (stdout: pages processed, links found, annotations extracted, errors)
- [x] 4.4 Ensure error details are logged to stderr (file path, error message)

## 5. CLI Command Implementation

- [x] 5.1 Add `Index` variant to `Commands` enum in `cli/src/main.rs`
- [x] 5.2 Define CLI args with `--vault <path>` and `--page <uuid>` flags
- [x] 5.3 Add vault discovery logic (Git-style from CWD or explicit --vault path)
- [x] 5.4 Wire up `cerbo index` command to call appropriate core indexing function
- [x] 5.5 Handle JSON output format and error reporting

## 6. Testing

- [x] 6.1 Add unit test: `index_page()` with links updates target backrefs
- [x] 6.2 Add unit test: `index_page()` with annotations writes annotations.ttl
- [x] 6.3 Add unit test: `index_vault()` is idempotent (run twice, same result)
- [x] 6.4 Add unit test: `index_page()` handles corrupted/missing files gracefully
- [x] 6.5 Add integration test: `cerbo index` indexes full vault with backrefs
- [x] 6.6 Add integration test: `cerbo index --page <uuid>` indexes single page
- [x] 6.7 Add integration test: `cerbo index --vault <path>` uses explicit path
- [x] 6.8 Add integration test: Git-style vault discovery from subdirectory

## 7. Documentation

- [ ] 7.1 Update CLI help text for `cerbo index` command
- [ ] 7.2 Add man page entry for `cerbo index` (if man pages exist)
- [ ] 7.3 Add usage examples to README or docs (all scopes: all/vault/page)
- [ ] 7.4 Document when to use `cerbo index` (recovery workflow after manual edits/imports)
