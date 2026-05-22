## Context

Cerbo stores pages in `.cerbo/objects/<uuid>/` with derived metadata in `backrefs.ttl` (who links here) and `annotations.ttl` (semantic annotations). These files are generated when pages are written via `page_write_with_links()` and `page_write_with_annotations()`.

**Current state:**
- Link extraction: `extract_cerbo_links()` finds `cerbo://<uuid>` patterns
- Annotation extraction: `extract_annotations()` finds `[text]{prefix:Type}` patterns
- Backref updates: `backrefs_add()` / `backrefs_remove()` maintain bidirectional links
- All updates happen during write operations only

**Problem:** If files are edited manually, imported in bulk, or corrupted during crashes, the metadata TTL files become stale. There's no recovery mechanism.

**Constraints:**
- Must use existing parsing logic (`extract_cerbo_links`, `extract_annotations`)
- Must preserve existing backref/annotation write logic (`backrefs_add`, `annotations_write`)
- CLI-only feature (no GUI needed initially)
- Must handle vaults with thousands of pages efficiently

## Goals / Non-Goals

**Goals:**
- Index all `backrefs.ttl` and `annotations.ttl` files from current page content
- Support three scopes: all pages (default), single vault (`--vault <uuid>`), single page (`--page <uuid>`)
- Provide progress feedback during indexing (page count, errors encountered)
- Idempotent operation (safe to run multiple times)

**Non-Goals:**
- Real-time metadata validation (this is a recovery tool, not a daemon)
- GUI integration (CLI-first, UI can be added later)
- Backup/restore of old metadata (destructive rebuild)
- Handling non-page objects (Products, Sources, Attachments don't have backrefs)

## Decisions

### 1. Scope-aware indexing algorithm

**Decision:** Support three scopes with different behaviors:
1. **No flags** (default): Index all pages across all vaults (two-pass: clear all backrefs, then rebuild)
2. **`--vault <uuid>`**: Index only pages within specified vault (two-pass within vault scope)
3. **`--page <uuid>`**: Index single page (incremental: update only this page's outgoing links and annotations)

**Rationale:**
- Default (all pages): Full consistency guarantee, clears orphaned backrefs
- Vault scope: Useful for large multi-vault setups (faster than full index)
- Page scope: Debugging tool, quick fix after manual edit (no need to reprocess entire vault)

**Alternative considered:** Always use two-pass for all scopes
- **Rejected:** Inefficient for single-page updates (would clear/rebuild all backrefs unnecessarily)

### 2. Reuse existing extraction functions

**Decision:** Call `extract_cerbo_links()` and `extract_annotations()` directly from the CLI command.

**Rationale:**
- No code duplication
- Same parsing logic as live writes (consistency)
- Already battle-tested

**Alternative considered:** Implement new parsing for CLI
- **Rejected:** Divergence risk, maintenance burden

### 3. CLI structure: `cerbo index [--vault <uuid>] [--page <uuid>]`

**Decision:** Top-level `index` command with optional scope filters.

**Rationale:**
- Shorter, more intuitive (mirrors common CLI patterns like `git index`)
- `--vault <uuid>` limits indexing to one vault
- `--page <uuid>` indexes a single page (useful for debugging or incremental fixes)
- No arguments = index all pages across all vaults

**Alternative considered:** `cerbo page rebuild-metadata`
- **Rejected:** Too verbose, "rebuild-metadata" is implementation detail, "index" is user-facing concept

### 4. Progress feedback via stderr

**Decision:** Print progress to stderr (e.g., "Processing 450/1200 pages...") and summary to stdout.

**Rationale:**
- Allows piping stdout for scripting
- User still sees progress in interactive mode

## Risks / Trade-offs

**[Risk]** Large vaults (10k+ pages) may take minutes to index  
→ **Mitigation:** Show progress bar, process in batches, use `--vault` or `--page` for targeted indexing

**[Risk]** Corrupted page.md files may cause parsing errors  
→ **Mitigation:** Log errors but continue processing remaining pages. Return non-zero exit code if any failures.

**[Risk]** Concurrent writes during indexing may cause conflicts  
→ **Mitigation:** Document that indexing should be run when app is not actively writing. Future: add vault locking.

**[Trade-off]** Two-pass approach means 2x disk reads  
→ **Accepted:** Simplicity and correctness over performance for a recovery tool

## Migration Plan

**Deployment:**
1. Add CLI command in `cli/src/main.rs` (new top-level `index` command)
2. No database migrations needed (operates on existing file structure)
3. No breaking changes (additive only)

**Rollback:**
- If indexing fails, user can restore `.cerbo/` from backup
- Recommend testing on a copy of the vault first

**Usage:**
```bash
# Index all pages across all vaults
cerbo index

# Index specific vault
cerbo index --vault <vault-uuid>

# Index single page
cerbo index --page <page-uuid>

# Dry-run mode (future enhancement)
cerbo index --dry-run
```

## Open Questions

1. Should we add a `--dry-run` flag to preview changes without writing? (Low priority, can add later)
2. Should we parallelize page processing? (Optimization for future, start with serial)
3. Should we backup old TTL files before overwriting? (Adds complexity, users can snapshot vault before rebuild)
