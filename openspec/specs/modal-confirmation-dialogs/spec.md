# Modal Confirmation Dialogs

## Purpose
Define the shared interaction model for modal confirmation dialogs.

## Requirements

### Requirement: Shared confirmation dialog pattern
All modal confirmation dialogs MUST use the same surface, spacing, action layout, and focus treatment as the current quit confirmation dialog.

### Requirement: Confirmation dialog keyboard behavior
When a modal confirmation dialog is open, only `Arrow` keys, `Enter`, and `Escape` MAY affect the dialog.

### Requirement: Confirmation dialog modal isolation
When a modal confirmation dialog is open, the dialog MUST prevent unrelated application hotkeys from triggering.

### Requirement: Delete confirmation parity
The delete-page confirmation dialog MUST present the same interaction model as the quit confirmation dialog.
