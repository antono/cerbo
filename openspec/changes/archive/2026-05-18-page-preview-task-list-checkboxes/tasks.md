## 1. Preview rendering

- [x] 1.1 Identify the preview markdown rendering path for task list items
- [x] 1.2 Remove bullet styling for `contains-task-list` items in preview
- [x] 1.3 Increase checkbox size with CSS without changing line height

## 2. Interactive checkbox behavior

- [x] 2.1 Add preview click handling for task list checkboxes
- [x] 2.2 Update rendered preview state immediately after toggle
- [x] 2.3 Preserve non-task preview interactions while handling checkbox clicks

## 3. Markdown persistence

- [x] 3.1 Update markdown source when a checkbox is toggled
- [x] 3.2 Route checkbox mutations through the existing page write/save flow
- [x] 3.3 Reload or synchronize page content after persistence succeeds

## 4. Verification

- [x] 4.1 Add or update tests for bullet-free rendering
- [x] 4.2 Add or update tests for checkbox toggling and markdown persistence
- [x] 4.3 Verify preview layout still preserves line height and spacing
