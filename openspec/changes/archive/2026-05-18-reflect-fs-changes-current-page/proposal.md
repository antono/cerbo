## Why

Users can edit page content both inside Cerbo and directly on disk, but the app does not yet define how the current page should react when the underlying `page.md` changes externally. This change prevents stale preview/edit state and makes external file edits predictable without affecting the rest of the app.

## What Changes

- Refresh the currently open page when its backing file changes on disk.
- In preview mode, re-render the current page from the updated file content.
- In edit mode, prompt the user to either load the external changes or keep the in-app version and overwrite the file.
- Keep the behavior scoped to the current page only; do not switch pages or refresh unrelated UI state.

## Capabilities

### New Capabilities
- `current-page-filesystem-sync`: synchronize the active page with external filesystem changes and resolve edit-mode conflicts.

### Modified Capabilities
- `page-crud`: page reading and editing behavior changes to account for external file modifications while a page is open.

## Impact

- Frontend editor state management for the active page.
- File watcher handling for the page currently open in the UI.
- Conflict dialog behavior when disk content diverges from the editor buffer.
- Preview rendering refresh path for updated markdown content.
