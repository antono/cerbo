## Why

Switching from edit mode to preview currently loses the editor's cursor
location, which makes it slow to return to the same spot after reviewing
rendered content. Persisting the cursor position per editor in local cache keeps
the edit-preview loop continuity and avoids forcing the user to re-find their
place.

## What Changes

- Save the current editor cursor position when the user leaves edit mode for
  preview.
- Store the position in `.cache/cerbo/...` metadata scoped to the current
  editor/page.
- Restore the saved cursor position when the user re-enters the editor.
- Scroll the editor to ensure the restored cursor is visible after restoration.
- If the saved cursor position is invalid or beyond the current file length,
  place the cursor on line 2 of the file.

## Capabilities

### New Capabilities

- `editor-cursor-position`: persist and restore the editor cursor position
  across edit/preview mode switches.

### Modified Capabilities

- None.

## Impact

- Editor mode switching behavior in the Svelte UI.
- Local metadata written under `.cache/cerbo/`.
- Cursor restoration logic when opening or reactivating an editor.
