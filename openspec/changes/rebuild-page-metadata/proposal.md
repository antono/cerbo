## Why

Page metadata (backrefs.ttl and annotations.ttl) can become stale or corrupted after manual file edits, bulk imports, or system crashes. There's currently no way to rebuild this derived metadata without re-importing pages. A CLI command to scan all pages and regenerate metadata ensures data consistency and enables recovery workflows.

## What Changes

- New CLI command `cerbo index` that:
  - Indexes all pages across all vaults (default behavior)
  - Parses each `page.md` for `cerbo://` links and `[text]{prefix:Type}` annotations
  - Regenerates `backrefs.ttl` for each linked page (who links to this page)
  - Regenerates `annotations.ttl` for each page containing annotations
  - Supports `--vault <uuid>` to index only pages within a specific vault
  - Supports `--page <uuid>` to index a single page
  - Provides progress feedback and summary statistics

## Capabilities

### New Capabilities
- `page-metadata-index`: CLI command that indexes/rebuilds backrefs and annotations from page content

### Modified Capabilities
<!-- No existing specs are being modified - this is a new maintenance/recovery capability -->

## Impact

**Affected code:**
- `core/src/page.rs` - May need to expose link/annotation parsing as public API
- `core/src/links.rs` - Backref regeneration logic
- `core/src/annotations.rs` - Annotation extraction and TTL generation
- `cli/src/main.rs` - New top-level `index` command

**API surface:**
- New public CLI command (non-breaking addition)
- May require refactoring existing parsing logic to be reusable
