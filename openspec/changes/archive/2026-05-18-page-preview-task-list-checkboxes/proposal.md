## Why

Task lists in page preview currently do not behave like editable markdown tasks. Users need preview to reflect checklist formatting accurately and allow toggling task state directly in the rendered page, with changes saved back to the markdown source.

## What Changes

- Remove bullet styling from rendered markdown task list items in page preview.
- Render task list checkboxes larger while preserving the current line height and layout.
- Make preview checkboxes interactive so users can toggle checked and unchecked states.
- Persist checkbox state changes back to the underlying markdown source.

## Capabilities

### New Capabilities

- `page-task-list-interaction`: interactive task list rendering and state persistence in page preview.

### Modified Capabilities

- `page-crud`: page preview and markdown save behavior now include interactive task list handling and checkbox state persistence.

## Impact

- Page preview rendering for markdown task lists.
- Markdown update flow for pages, including save/write behavior.
- Editor interactions that mutate rendered markdown content.
- Any preview styling or markdown renderer plugins that currently assume task list items are static.
