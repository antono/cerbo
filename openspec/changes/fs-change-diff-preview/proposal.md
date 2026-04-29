## Why

External file changes currently force a binary choice: load the disk version or overwrite it from the editor. That works for simple conflicts, but it makes it hard to inspect what actually changed before deciding.

## What Changes

- Add an on-demand diff preview for file-system change conflicts.
- Show the preview by re-reading the file from disk and diffing it against the current in-memory editor content.
- Expand the current conflict dialog into a larger diff-focused view when the user requests preview.
- Keep the existing actions available in the diff view so the user can approve the disk changes or overwrite them from the editor.
- Preserve the lightweight prompt as the default entry point; the diff view remains optional.
- Keep the diff output shaped for future git-backed history and page operations.

## Capabilities

### New Capabilities
- `fs-change-diff-preview`: inspect external file changes on demand and approve or reject them from a diff view.

### Modified Capabilities
- None.

## Impact

- `src/lib/ExternalChangeDialog.svelte` will need a preview state and a larger diff presentation.
- `src/lib/PageEditor.svelte` will need to route external change conflicts into the new preview flow.
- `src/lib/page-sync.ts` will need diff generation that can support the UI preview.
- Future git integration should align with the same diff shape so the preview can evolve into history-backed views later.
