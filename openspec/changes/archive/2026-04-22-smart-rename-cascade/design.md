## Context

The current `page_rename` implementation in `core/src/rename.rs` performs a full scan of the vault to update wikilinks. This is O(N) where N is the number of pages in the vault. While functional for small vaults, it becomes a performance bottleneck for larger datasets.

The `cerbo-core` already has a link indexing mechanism (backlinks) used by the UI. This index can be leveraged to reduce the search space for renames to only the pages that are known to link to the target.

## Goals / Non-Goals

**Goals:**
- Improve the performance of page renaming by using the link index for discovery.
- Ensure 100% correctness of the rename cascade (no broken links).
- Implement robust testing in both `cerbo-core` and the CLI.
- Create a reusable fixture vault for testing complex link scenarios.

**Non-Goals:**
- Changing the wikilink syntax.
- Implementing automatic redirect files (out of scope for this optimization).

## Decisions

### 1. Use `index::compute_backlinks` for Link Discovery
Instead of `std::fs::read_dir` in `rename.rs`, we will load the index and call `compute_backlinks` to get a list of slugs that need updating.

### 2. Standardized Testing Fixture
A new module (or helper) in `core` will provide a `create_fixture_vault` function.
This fixture will contain:
- Page A: Links to B
- Page B: Target of rename
- Page C: Links to B (case-insensitive variant)
- Page D: No links to B
- Page E: Links to B and A

### 3. CLI Integration Tests
We will add a test suite that invokes the CLI binary (or its main logic) against the fixture vault to verify end-to-end correctness of the rename command.

## Risks / Trade-offs

- **Risk**: Stale index leads to missed link updates.
- **Mitigation**: Ensure `page_rename` forces an index check/rebuild before discovery, or handles index misses gracefully by falling back to full scan if the index is missing.
- **Trade-off**: Memory usage for the index during rename. For very large vaults, loading the index is still better than reading all files from disk.
