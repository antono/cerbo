## 1. Persist cursor metadata

- [ ] 1.1 Identify the active editor/page key used for local cache entries under `.cache/cerbo/`
- [ ] 1.2 Save cursor line and column when switching from edit mode to preview mode
- [ ] 1.3 Load saved cursor metadata when opening the editor for the same page

## 2. Restore editor position

- [ ] 2.1 Restore the saved cursor position when the editor becomes active
- [ ] 2.2 Scroll the editor so the restored cursor is visible
- [ ] 2.3 Fall back to line 2 when the saved position is missing or out of range

## 3. Verify behavior

- [ ] 3.1 Add or update tests for save-on-preview behavior
- [ ] 3.2 Add or update tests for restore-and-scroll behavior
- [ ] 3.3 Add or update tests for out-of-range fallback to line 2
