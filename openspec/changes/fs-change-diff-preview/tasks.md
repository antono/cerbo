## 1. Modal Flow

- [x] 1.1 Add a preview state to the external-change dialog
- [x] 1.2 Wire the dialog so preview opens the expanded diff view on demand
- [x] 1.3 Keep the existing `Load changes` and `Overwrite` actions available in both states
- [x] 1.4 Keep the preview header compact and the preview surface opaque
- [x] 1.5 Keep the preview-specific controls out of the expanded diff view
- [x] 1.6 Ensure the modal surface fully covers the sidebar chrome beneath it

## 2. Diff Generation and Rendering

- [x] 2.1 Re-read the current page file from disk when preview is requested
- [x] 2.2 Generate a standard unified diff from disk content versus editor buffer
- [x] 2.3 Render the diff in the expanded preview view using `git-diff-view`
- [x] 2.4 Remove redundant refresh controls from the expanded preview view

## 3. Conflict Resolution Behavior

- [x] 3.1 Ensure `Load changes` applies the current disk content and closes the conflict UI
- [x] 3.2 Ensure `Overwrite` writes the editor buffer and closes the conflict UI
- [x] 3.3 Preserve the existing conflict suppression behavior after overwrite

## 4. Verification

- [x] 4.1 Add or update tests for on-demand preview flow
- [x] 4.2 Add or update tests for live disk reread behavior
- [x] 4.3 Run the relevant checks for the modal and page-sync flow
