# Proposal: UI Backlinks Panel — Fix UUID Storage Model Compatibility

## Problem Statement

After the migration from slug-based to UUID-based object storage (`.cerbo/objects/<uuid>/`), the backlinks panel disappeared from the UI. The backend correctly implements UUID-based backlink resolution via `backrefs.ttl`, but the frontend UI components were using misleading prop names (`slug` instead of `uuid`) while actually passing UUID values, causing confusion and potential bugs.

## Motivation

The UUID-based storage model is the foundation for semantic capabilities and proper link tracking. The UI must correctly expose backlinks to users so they can understand page relationships and navigate between linked pages. Prop naming clarity is essential for maintainability as the system evolves.

## Proposed Solution

Rename UI component props from `slug` to `uuid` to accurately reflect that they receive UUID values in the new storage model. This aligns frontend prop naming with:
- Backend function signatures (`loadBacklinks(uuid)`, `loadAttachments(uuid)`)
- Storage spec terminology (`.cerbo/objects/<uuid>/`)
- Tauri command parameters (`backlinks_get { vaultId, uuid }`)

## Capabilities

### New Capabilities
None — this is a bug fix, not new functionality.

### Modified Capabilities
- `backlinks`: No spec change. Implementation fix: prop naming clarity. Panel now correctly loads and displays backlinks for the current page by UUID.
- `page-attachments`: No spec change. Implementation fix: prop naming clarity. Attachments panel now correctly loads and manages attachments using UUID instead of slug.

## Impact

**Affected code:**
- `src/lib/RightSidebarPanel.svelte` — accepts `uuid` prop, calls `loadBacklinks(uuid)`
- `src/lib/AttachmentsPanel.svelte` — accepts `uuid` prop, calls `loadAttachments(uuid)` and attachment Tauri commands with UUID
- `src/routes/+page.svelte` — passes `uuid={app.currentUuid}` to both panels

**No API changes:**
- Backend Tauri commands already use UUID correctly
- `backrefs.ttl` structure unchanged (per uuid-object-storage spec)
- Backlinks display contract unchanged (per backlinks spec: `{ uuid, title }` entries)

**No user-facing changes:**
- Backlinks panel display and interaction unchanged
- Attachments panel display and interaction unchanged

## Success Criteria

- [x] RightSidebarPanel prop renamed `slug` → `uuid`
- [x] AttachmentsPanel prop renamed `slug` → `uuid`  
- [x] Parent component (+page.svelte) passes UUID with correct prop names
- [ ] App builds without TypeScript errors
- [ ] Backlinks panel displays correctly when opening a page with backlinks
- [ ] Attachments panel displays and manages attachments correctly
