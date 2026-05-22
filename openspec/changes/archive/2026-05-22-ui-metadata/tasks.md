# Tasks: UI Metadata – Custom Slug and Virtual Path

## 1. Setup

- [x] 1.1 Create slug generation utility function (transform page name to URL-friendly slug)
- [x] 1.2 Identify existing new-page dialog component location and review current structure

## 2. Form Component Enhancement

- [x] 2.1 Add slug input field to new-page dialog form (between page name and virtual path)
- [x] 2.2 Add virtual path input field to new-page dialog form
- [x] 2.3 Add form state properties: `slug`, `virtualPath`, `slugAutoUpdateEnabled`

## 3. Slug Auto-Update Logic

- [x] 3.1 Implement page name change listener that triggers slug generation when `slugAutoUpdateEnabled` is true
- [x] 3.2 Implement slug field change listener to set `slugAutoUpdateEnabled` to false on first edit
- [x] 3.3 Initialize slug field with generated value when dialog opens

## 4. Virtual Path Autocomplete

- [x] 4.1 Fetch existing paths from vault context when dialog opens
- [x] 4.2 Implement autocomplete suggestion logic (case-insensitive prefix matching on virtual path input)
- [x] 4.3 Wire autocomplete UI to display and select suggestions on keystroke

## 5. Client-Side Validation

- [x] 5.1 Add slug validation: only alphanumeric, hyphens, underscores (required, non-empty)
- [x] 5.2 Add virtual path validation: alphanumeric, hyphens, underscores, forward slashes (required, non-empty)
- [x] 5.3 Show error messages for invalid inputs and prevent form submission

## 6. Form Submission

- [x] 6.1 Update form submit handler to pass `slug` and `virtualPath` to page creation API call
- [x] 6.2 Test form submission with valid slug and path values

## 7. Testing & Polish

- [x] 7.1 Manual test: slug auto-updates as page name is typed
- [x] 7.2 Manual test: slug auto-update stops after manual edit
- [x] 7.3 Manual test: autocomplete works with existing vault paths
- [x] 7.4 Manual test: validation blocks submission with invalid slug or path
- [x] 7.5 Manual test: multiple dialogs maintain independent state
- [x] 7.6 Visual indicator for disabled auto-update state (optional enhancement)
