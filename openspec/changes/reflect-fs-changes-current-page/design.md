## Context

Cerbo currently supports editing and previewing a page, plus watching vault files for changes. This change adds a sharper rule for what happens when the backing `page.md` changes outside the app while the current page is open. The behavior must stay local to the active page and avoid broad refreshes that would disrupt navigation or unrelated editor state.

## Goals / Non-Goals

**Goals:**
- Keep the active page synchronized with external filesystem edits.
- Re-render preview mode immediately from updated file content.
- Prevent silent data loss in edit mode by forcing an explicit choice when disk and editor content diverge.
- Limit updates to the currently open page only.

**Non-Goals:**
- Live sync for pages that are not currently open.
- Background merging of concurrent edits.
- Changing the underlying vault watcher model beyond current-page dispatch.
- Introducing version history or conflict resolution beyond load/overwrite.

## Decisions

1. Drive refresh behavior from the existing file watcher, but scope the effect to the active page only.
   - This keeps the implementation aligned with the current filesystem monitoring path instead of adding a separate sync subsystem.
   - Alternative: a second watcher just for the active page. Rejected because it duplicates filesystem monitoring and adds lifecycle complexity.

2. Treat preview mode as authoritative display state and reload it from disk on change.
   - Preview mode is read-only, so the simplest safe behavior is to discard cached render state and re-read the file.
   - Alternative: incremental patching of rendered output. Rejected because the source of truth is the file, and full re-read is simpler and less error-prone.

3. In edit mode, require a user decision when the current buffer differs from disk.
   - The dialog must make the conflict explicit and offer only two outcomes: load disk changes or overwrite disk with the editor buffer.
   - Alternative: auto-merge or auto-reload. Rejected because the user asked for an explicit choice and silent resolution risks data loss.

4. Keep the conflict decision at the page editor layer rather than global app state.
   - The behavior is page-specific and should not affect other open views or navigation state.
   - Alternative: central app-level modal coordination. Rejected because it broadens the blast radius without adding value for a single-page conflict.

## Risks / Trade-offs

- [Frequent file-change events could cause repeated reloads] → Debounce or coalesce events for the active page before re-rendering or showing a dialog.
- [Conflict dialog can interrupt editing flow] → Trigger it only when the open page is both active and dirty relative to disk.
- [Reloading preview may reset transient UI state inside the page view] → Keep the refresh scoped to content rendering, not navigation or vault selection.
- [Overwrite choice can still discard unsaved changes] → Label the dialog clearly so the user understands the consequence before confirming.
