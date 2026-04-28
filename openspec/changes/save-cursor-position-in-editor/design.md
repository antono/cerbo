## Context

Cerbo already switches between edit and preview modes for a page, but it does
not preserve editor viewport state across that transition. The new behavior
needs to persist cursor position locally per editor so users can resume editing
where they left off after previewing rendered content.

The state must live in the app's local cache under `.cache/cerbo/` and be scoped
to the current editor/page. On restore, the editor should both place the cursor
and scroll so that cursor is visible. If the saved position no longer fits the
file, the editor should fall back to line 2.

## Goals / Non-Goals

**Goals:**

- Persist cursor position when leaving edit mode for preview.
- Restore cursor position when returning to edit mode.
- Ensure the restored cursor is visible in the viewport.
- Fall back safely when the stored position is stale or invalid.

**Non-Goals:**

- Persisting full editor session state beyond cursor position.
- Syncing cursor state across devices or vaults.
- Changing page content or preview rendering behavior.

## Decisions

- Store cursor metadata in the existing local cache area under `.cache/cerbo/`
  rather than page content or vault files. This keeps editor state ephemeral and
  avoids polluting user documents.
- Key the saved position by the current editor/page identity so each page
  restores independently. This avoids cross-page cursor leakage when switching
  between files.
- Restore cursor position when the editor becomes active again, then scroll to
  the cursor rather than trying to infer a saved scroll offset. Cursor
  visibility is the user-facing requirement; explicit scroll restoration is
  unnecessary.
- Treat out-of-range positions as stale data and recover by placing the cursor
  on line 2. Line 1 is reserved for file structure and line 2 is a safer default
  for immediate continuation in typical markdown files.

## Risks / Trade-offs

- Saved positions can become invalid after file edits -> Validate against
  current file length before restore and apply the line-2 fallback.
- Re-entering edit mode may race with editor initialization -> Apply restoration
  only after the editor instance is ready.
- Line 2 fallback may still point to a non-editable or blank region in unusual
  files -> This is acceptable as a deterministic recovery point and can be
  revisited if needed.

## Migration Plan

No data migration is needed. New cursor metadata can be created on demand the
first time a user leaves edit mode.

Rollback is straightforward: stop writing and reading the cursor metadata,
leaving any existing cache entries unused.

## Open Questions

- None at this time.
