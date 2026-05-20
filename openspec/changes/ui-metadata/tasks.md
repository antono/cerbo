# Tasks: UI Metadata – Custom Slug and Virtual Path

## 1. Setup

- [ ] 1.1 Create slug generation utility function (transform page name to URL-friendly slug)
- [ ] 1.2 Identify existing new-page dialog component location and review current structure

## 2. Form Component Enhancement

- [ ] 2.1 Add slug input field to new-page dialog form (between page name and virtual path)
- [ ] 2.2 Add virtual path input field to new-page dialog form
- [ ] 2.3 Add form state properties: `slug`, `virtualPath`, `slugAutoUpdateEnabled`

## 3. Slug Auto-Update Logic

- [ ] 3.1 Implement page name change listener that triggers slug generation when `slugAutoUpdateEnabled` is true
- [ ] 3.2 Implement slug field change listener to set `slugAutoUpdateEnabled` to false on first edit
- [ ] 3.3 Initialize slug field with generated value when dialog opens

## 4. Virtual Path Autocomplete

- [ ] 4.1 Fetch existing paths from vault context when dialog opens
- [ ] 4.2 Implement autocomplete suggestion logic (case-insensitive prefix matching on virtual path input)
- [ ] 4.3 Wire autocomplete UI to display and select suggestions on keystroke

## 5. Client-Side Validation

- [ ] 5.1 Add slug validation: only alphanumeric, hyphens, underscores (required, non-empty)
- [ ] 5.2 Add virtual path validation: alphanumeric, hyphens, underscores, forward slashes (required, non-empty)
- [ ] 5.3 Show error messages for invalid inputs and prevent form submission

## 6. Form Submission

- [ ] 6.1 Update form submit handler to pass `slug` and `virtualPath` to page creation API call
- [ ] 6.2 Test form submission with valid slug and path values

## 7. Testing & Polish

- [ ] 7.1 Manual test: slug auto-updates as page name is typed
- [ ] 7.2 Manual test: slug auto-update stops after manual edit
- [ ] 7.3 Manual test: autocomplete works with existing vault paths
- [ ] 7.4 Manual test: validation blocks submission with invalid slug or path
- [ ] 7.5 Manual test: multiple dialogs maintain independent state
- [ ] 7.6 Visual indicator for disabled auto-update state (optional enhancement)
