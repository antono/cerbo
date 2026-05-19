# Specs: UI Backlinks Panel — Fix UUID Storage Model Compatibility

## Status

This change is an **implementation bug fix** with **no spec requirement changes**. All relevant capabilities already have comprehensive specs that correctly describe the behavior. This change ensures the implementation conforms to those existing specs.

## Related Existing Specs

### `backlinks` capability
**Location:** `openspec/specs/backlinks/spec.md`

**What it specifies:**
- Display backlinks panel for the current page, listing all pages that link to it
- Panel lists entries identified by `uuid` and `title` (no slug)
- Each entry is navigable by UUID
- Backlinks computed from `.cerbo/objects/<uuid>/backrefs.ttl`
- Tauri command `backlinks_get` resolves each UUID to `{ uuid, title }`

**Status:** ✓ Already correct. Implementation was not conforming; now fixed.

### `page-attachments` capability  
**Location:** `openspec/specs/page-attachments/spec.md`

**What it specifies:**
- Load and display attachments for the current page
- Support delete, open, and insert-link operations
- Use UUID to reference pages and objects

**Status:** ✓ Already correct. Implementation was not conforming; now fixed.

### `uuid-object-storage` capability
**Location:** `openspec/specs/uuid-object-storage/spec.md`

**What it specifies:**
- Objects stored under `.cerbo/objects/<uuid>/`
- `meta.ttl` and `backrefs.ttl` follow Turtle RDF format
- All links and references use UUID

**Status:** ✓ Already correct. No changes needed.

## ADDED Requirements

None. All capability specs already exist and are unchanged.

## REMOVED Requirements

None. All capability specs remain in effect.

## MODIFIED Requirements

None. No spec-level behavior changes — only implementation fix to align with existing specs.
