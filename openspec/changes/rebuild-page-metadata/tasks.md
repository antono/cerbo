## 1. Core Library Refactoring

- [ ] 1.1 Make `extract_cerbo_links()` public in `core/src/links.rs`
- [ ] 1.2 Make `extract_annotations()` public in `core/src/annotations.rs`
- [ ] 1.3 Add `backrefs_clear()` function in `core/src/links.rs` to reset a vault's backrefs
- [ ] 1.4 Add public API in `core/src/lib.rs` for indexing operations

## 2. Vault/Page Discovery

- [ ] 2.1 Add `list_all_vaults()` function in `core/src/vault.rs` to enumerate all vaults
- [ ] 2.2 Add `list_pages_in_vault(vault_uuid)` function to enumerate pages in a vault
- [ ] 2.3 Add `get_page_path(page_uuid)` helper to resolve page object directory

## 3. Indexing Logic

- [ ] 3.1 Create `core/src/index.rs` module for indexing operations
- [ ] 3.2 Implement `index_all_pages(ctx)` - two-pass algorithm (clear all, rebuild all)
- [ ] 3.3 Implement `index_vault(ctx, vault_uuid)` - two-pass within vault scope
- [ ] 3.4 Implement `index_page(ctx, page_uuid)` - incremental single-page indexing
- [ ] 3.5 Add error handling for corrupted/missing page.md files (log and continue)
- [ ] 3.6 Add warning logging for broken link references (nonexistent UUIDs)

## 4. Progress Feedback

- [ ] 4.1 Create progress tracking struct (total pages, processed count, errors)
- [ ] 4.2 Add periodic progress output to stderr ("Processing N/M pages...")
- [ ] 4.3 Add summary statistics on completion (stdout: pages processed, links found, annotations extracted, errors)
- [ ] 4.4 Ensure error details are logged to stderr (file path, error message)

## 5. CLI Command Implementation

- [ ] 5.1 Add `Index` variant to `Commands` enum in `cli/src/main.rs`
- [ ] 5.2 Define CLI args struct with optional `--vault <uuid>` and `--page <uuid>` flags
- [ ] 5.3 Add mutual exclusivity validation (error if both --vault and --page provided)
- [ ] 5.4 Wire up `cerbo index` command to call appropriate core indexing function
- [ ] 5.5 Handle exit codes (0 for success, non-zero if any errors encountered)

## 6. Testing

- [ ] 6.1 Add unit test: `index_page()` with links updates target backrefs
- [ ] 6.2 Add unit test: `index_page()` with annotations writes annotations.ttl
- [ ] 6.3 Add unit test: `index_all_pages()` is idempotent (run twice, same result)
- [ ] 6.4 Add integration test: `cerbo index` with no args indexes all pages
- [ ] 6.5 Add integration test: `cerbo index --vault <uuid>` scopes to vault
- [ ] 6.6 Add integration test: `cerbo index --page <uuid>` indexes single page
- [ ] 6.7 Add integration test: `cerbo index --vault <uuid> --page <uuid>` exits with error
- [ ] 6.8 Add integration test: corrupted page.md logs error but continues

## 7. Documentation

- [ ] 7.1 Update CLI help text for `cerbo index` command
- [ ] 7.2 Add man page entry for `cerbo index` (if man pages exist)
- [ ] 7.3 Add usage examples to README or docs (all scopes: all/vault/page)
- [ ] 7.4 Document when to use `cerbo index` (recovery workflow after manual edits/imports)
