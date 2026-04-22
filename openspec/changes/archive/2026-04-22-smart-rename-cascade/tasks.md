## 1. Testing Infrastructure

- [x] 1.1 Create `core/src/fixtures.rs` with `create_fixture_vault` helper.
- [x] 1.2 Implement complex link scenarios in the fixture (case-insensitivity, multiple links).

## 2. Core Implementation

- [x] 2.1 Refactor `core/src/rename.rs`: modify `page_rename` to load the index before cascading.
- [x] 2.2 Refactor `core/src/rename.rs`: replace `rewrite_links_in_vault` with index-driven discovery using `compute_backlinks`.
- [x] 2.3 Ensure index is saved after the rename operation (already part of `page_rename` but needs verification).

## 3. Validation and Integration

- [x] 3.1 Add unit tests in `core/src/rename.rs` using the new fixture.
- [x] 3.2 Create CLI integration test suite (possibly in a new test file or `cli/src/main.rs` tests).
- [x] 3.3 Verify end-to-end rename via CLI on the fixture vault.
