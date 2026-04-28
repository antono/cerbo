## 1. Persist cursor metadata

- [x] 1.1 Identify the active editor/page key used for local cache entries under `.cache/cerbo/`
- [x] 1.2 Save cursor line and column when switching from edit mode to preview mode
- [x] 1.3 Load saved cursor metadata when opening the editor for the same page

## 2. Restore editor position

- [x] 2.1 Restore the saved cursor position when the editor becomes active
- [x] 2.2 Scroll the editor so the restored cursor is visible
- [x] 2.3 Fall back to line 2 when the saved position is missing or out of range

## 3. Verify behavior

- [x] 3.1 Add or update tests for save-on-preview behavior
- [x] 3.2 Add or update tests for restore-and-scroll behavior
- [x] 3.3 Add or update tests for out-of-range fallback to line 2
