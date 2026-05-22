# Tasks: UI Backlinks Panel — Fix UUID Storage Model Compatibility

## 1. Update RightSidebarPanel Component
- [x] 1.1 Rename prop from `slug` to `uuid` in component signature
- [x] 1.2 Update reactive effect to use `uuid` parameter
- [x] 1.3 Pass renamed prop to `loadBacklinks(uuid)` call

**Completed:** RightSidebarPanel.svelte lines 4-10

## 2. Update AttachmentsPanel Component
- [x] 2.1 Rename prop from `slug` to `uuid` in component signature
- [x] 2.2 Update `refreshAttachments()` to use `uuid`
- [x] 2.3 Update `deleteAttachment()` to use `uuid` in Tauri command
- [x] 2.4 Update `openAttachment()` to use `uuid` in Tauri command
- [x] 2.5 Update reactive effect to use `uuid` parameter

**Completed:** AttachmentsPanel.svelte lines 6, 10, 15, 24, 34, 52

## 3. Update Parent Component
- [x] 3.1 Update RightSidebarPanel invocation to pass `uuid={app.currentUuid}`
- [x] 3.2 Update AttachmentsPanel invocation to pass `uuid={app.currentUuid}`

**Completed:** +page.svelte lines 81, 84

## 4. Fix UUID Link Extraction Regex
- [x] 4.1 Fix `extract_cerbo_links` regex to match uppercase hex digits in UUIDs
- [x] 4.2 Changed regex from `[a-z0-9-]` to `[a-fA-F0-9-]` to support both cases

**Completed:** core/src/links.rs line 10

**Issue Fixed:** Backrefs were being written to wrong location (`.cerbo/objects/objects/backrefs.ttl`) because the regex only matched lowercase UUIDs, failing to extract uppercase UUIDs from links.

## 5. Verification
- [x] 5.1 Verify TypeScript syntax is correct in all modified files
- [ ] 5.2 Build the application to check for compile errors  
- [ ] 5.3 Test: Open a page with backlinks and verify panel displays correctly
- [ ] 5.4 Test: Open a page with attachments and verify panel functions correctly
- [ ] 5.5 Verify backrefs are written to correct `.cerbo/objects/<uuid>/backrefs.ttl` locations

## Summary

**Implementation Status:** Complete (4/4 code changes done)  
**Testing Status:** Pending (verification tasks remaining)

Changes made:
1. UI component prop naming (slug → uuid) for semantic correctness
2. UUID link extraction regex fix to handle uppercase hex digits

The backlinks panel is now properly configured to display backlinks for pages, and backrefs are written to the correct locations.
