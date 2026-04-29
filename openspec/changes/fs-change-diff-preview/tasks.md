## 1. Modal Flow

- [ ] 1.1 Add a preview state to the external-change dialog
- [ ] 1.2 Wire the dialog so preview opens the expanded diff view on demand
- [ ] 1.3 Keep the existing `Load changes` and `Overwrite` actions available in both states

## 2. Diff Generation and Rendering

- [ ] 2.1 Re-read the current page file from disk when preview is requested
- [ ] 2.2 Generate a standard unified diff from disk content versus editor buffer
- [ ] 2.3 Render the diff in the expanded preview view using `git-diff-view`

## 3. Conflict Resolution Behavior

- [ ] 3.1 Ensure `Load changes` applies the current disk content and closes the conflict UI
- [ ] 3.2 Ensure `Overwrite` writes the editor buffer and closes the conflict UI
- [ ] 3.3 Preserve the existing conflict suppression behavior after overwrite

## 4. Verification

- [ ] 4.1 Add or update tests for on-demand preview flow
- [ ] 4.2 Add or update tests for live disk reread behavior
- [ ] 4.3 Run the relevant checks for the modal and page-sync flow
