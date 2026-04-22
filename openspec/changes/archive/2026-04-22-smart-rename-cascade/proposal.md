## Why

The current implementation of the rename cascade walks the entire vault, reading every `page.md` file to find and replace wikilinks. As vaults grow, this becomes increasingly inefficient. By leveraging the existing link index (backlinks database), we can identify exactly which files need modification, making the rename operation significantly faster and more scalable.

## What Changes

- Refactor the rename cascade logic in `cerbo-core` to use the link index for identifying affected pages.
- Ensure the link index is rebuilt or updated correctly during and after the rename operation to maintain vault integrity.
- Update existing specifications to reflect this optimized approach to link maintenance.

## Capabilities

### New Capabilities
- `smart-rename-cascade`: Logic for targeted link rewriting using index-driven discovery.

### Modified Capabilities
- `rename-cascade`: Update requirement to use link index for discovery of pages to update instead of full-vault scan.
- `backlinks`: Update requirement to ensure the index is reliable for use by system maintenance commands like rename cascade.

## Impact

- `core/src/rename.rs`: `rewrite_links_in_vault` will be replaced or refactored to use `index::compute_backlinks`.
- `core/src/index.rs`: Minor updates to ensure indexing supports efficient discovery for renames (e.g., handling case-insensitivity consistently).
- `core/src/lib.rs` / `src-tauri/src/rename.rs`: Internal command flow remains the same but implementation becomes index-aware.
