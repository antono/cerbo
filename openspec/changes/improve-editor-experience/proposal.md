# Proposal

## Why

Writing in the desktop editor feels slow and leaky: typed characters appear
with a visible delay, linking another object needs an exact `[[` incantation,
dropping a file into a page does nothing (the upload command is a stub), and
reopening a page throws the caret back to line 2 (cursor save/load are stubs).
The vault selector also hides where each vault lives on disk, so two vaults with
the same name cannot be told apart.

## What Changes

- **Typing latency**: keystrokes in write mode are painted within one frame;
  heavy work (syntax overlay, markdown render of the hidden preview, autocomplete
  scanning) leaves the keystroke path.
- **Link picker on `[`**: typing `[` opens a dropdown with a search field;
  results filter instantly as the user types (title, type, virtual path).
  Selecting inserts `[Title](cerbo://objects/<uuid>)`. The existing `[[` and
  `(cerbo://` triggers keep working.
- **Vault paths in the selector**: every vault row shows its full filesystem
  path next to the name.
- **Drop into the editor**: a dropped file or dragged fragment is copied into the
  current vault as an `Attachment` object, shows up in the current page's
  attachments, and is inserted at the drop point — `![name](cerbo://objects/<uuid>)`
  for images, `[name](cerbo://objects/<uuid>)` for everything else. Pasting a
  file or image from the clipboard (Ctrl+V) does the same at the caret.
- **Return to the last place**: entering a page's editor puts the caret (and
  scroll) where the last editing session on that page ended, across page
  switches and app restarts.

## Capabilities

### New Capabilities

- `editor-input-performance`: latency budget for typing in the page editor.
- `editor-attachment-drop`: drag-and-drop and clipboard paste of files and
  fragments into the editor.
- `editor-position-restore`: persisting and restoring caret and scroll per page.

### Modified Capabilities

- `cerbo-link-autocomplete`: adds the `[`-triggered searchable link picker with
  instant filtering.
- `vault-selector`: vault rows show the full filesystem path.

## Non-goals

- Replacing Carta with another editor (CodeMirror, ProseMirror). Considered only
  as a fallback if profiling shows Carta cannot meet the budget.
- Fuzzy ranking with typo tolerance; substring/prefix matching is enough now.
- Deduplicating identical dropped files, or dropping onto the preview tab.
- Syncing cursor positions between machines (stored in local app config).

## Impact

- Frontend: `src/lib/PageEditor.svelte`, `src/lib/WikilinkAutocomplete.svelte`,
  `src/lib/cursor-position.ts`, `src/routes/+layout.svelte` (vault selector).
- Tauri commands: `attachment_upload` (stub → real, for in-memory fragments),
  reuse of the existing path-based `attachment_add` for OS file drops,
  `cursor_position_save` / `cursor_position_load` (stubs → real).
- Core: a bytes-based variant of `object::attachment_add`; the page's
  attachment list already derives from `cerbo://` links in `page.md`
  (`object::attachment_list`), so inserting the link is what attaches it.
  Cursor positions persisted in local app config next to `ui.toml`.
- No new dependencies expected.
