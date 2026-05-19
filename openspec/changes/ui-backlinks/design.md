# Design: UI Backlinks Panel — Fix UUID Storage Model Compatibility

## Context

The UUID-based storage model migration introduced `.cerbo/objects/<uuid>/` as the canonical object storage layout. The backlinks feature depends on:
- Backend: `backlinks_get` Tauri command reads `backrefs.ttl` and resolves UUIDs to `{ uuid, title }` entries
- Frontend: `RightSidebarPanel` and `AttachmentsPanel` components receive a UUID and load related data

The bug occurred because these UI components used the prop name `slug` while actually receiving UUID values, creating a semantic mismatch that confused the codebase and left the backlinks panel non-functional.

## Goals

**Goals:**
- Rename UI component props to accurately reflect UUID-based storage (`slug` → `uuid`)
- Ensure backlinks panel loads and displays correctly for the current page
- Ensure attachments panel loads and manages attachments correctly
- Maintain consistency between frontend prop names and backend function signatures

**Non-Goals:**
- Change backlinks display logic or UX (already spec-compliant)
- Modify Tauri command signatures (already correct)
- Add new backlinks features or filtering
- Alter attachments management logic

## Decisions

### Decision 1: Prop Naming Reflects Storage Model
**Decision:** Rename `slug` prop to `uuid` in both RightSidebarPanel and AttachmentsPanel.

**Rationale:** 
- Backend stores objects under `.cerbo/objects/<uuid>/` (per uuid-object-storage spec)
- All backend functions use `uuid` parameter names
- Current prop name `slug` contradicts the actual UUID value being passed
- Clear naming prevents future confusion and aligns with codebase intent

**Alternatives considered:**
- Keep `slug` name but add comments — rejected because it doesn't resolve the semantic error
- Create adapter layer — rejected as over-engineering for a naming correction

### Decision 2: Update Parent Component
**Decision:** Update `+page.svelte` to pass `uuid={app.currentUuid}` instead of `slug={app.currentUuid}`.

**Rationale:** Consistency with the renamed props. The parent already stores the UUID in `app.currentUuid`, so passing it with the correct prop name is straightforward.

### Decision 3: No Backend Changes
**Decision:** Backend code requires no changes.

**Rationale:** 
- `backlinks_get` already accepts UUID and returns `{ uuid, title }` entries
- `attachment_list`, `attachment_delete`, `attachment_open` already use UUID
- `loadBacklinks()` and `loadAttachments()` in stores already take UUID

## Risks / Trade-offs

### Risk: TypeScript Compilation
**Risk:** Renaming props could introduce type errors if not updated consistently.  
**Mitigation:** Update all references in files using these props (2 component files, 1 parent file). Verify with TypeScript type checking.

### Risk: Missed References
**Risk:** Other components might reference the old prop name.  
**Mitigation:** Search codebase for `slug` references in context of these components before declaring complete.

**Trade-off:** The fix is minimal and focused, affecting only prop names, not behavior. No test suite updates needed.
