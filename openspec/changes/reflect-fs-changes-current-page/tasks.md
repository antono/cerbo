## 1. Current-page file change routing

- [x] 1.1 Trace filesystem watcher events to the active page only and identify where the current page context is stored.
- [x] 1.2 Add current-page scoping so external `page.md` updates only affect the open page.

## 2. Preview refresh behavior

- [x] 2.1 Re-read the current page from disk when a watched change arrives in preview mode.
- [x] 2.2 Re-render the preview without switching pages or mutating unrelated UI state.

## 3. Edit-mode conflict handling

- [x] 3.1 Detect when the active editor buffer diverges from disk after an external file change.
- [x] 3.2 Show a dialog that lets the user load external changes or overwrite the file.
- [x] 3.3 Apply the selected action and keep the current page selected.

## 4. Verification

- [x] 4.1 Add or update tests for preview reload behavior on external file changes.
- [x] 4.2 Add or update tests for edit-mode conflict prompting and chosen action handling.
- [ ] 4.3 Add or update tests for ignoring unchanged filesystem change events.
