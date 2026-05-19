# Design: Replace Wikilinks with Cerbo UUID-based Links

## Context

### Current State
- Editor currently supports `[[Page Name]]` wikilink syntax via autocomplete
- Links are resolved by name matching during render phase
- Vault uses UUID-based storage model in the core layer
- Current link resolution is fragile—renames break links
- No existing cerbo:// URL scheme support in editor or renderer

### Constraints
- Must maintain backward compatibility during transition (old wikilinks should still render)
- Editor is Svelte-based SvelteKit frontend with markdown rendering
- Core has `cerbo resolve` command to map UUIDs to paths
- Desktop app (Tauri) and web editor must both support the feature

### Stakeholders
- Users editing vault pages
- Rendering pipeline (both desktop and web)
- Core library (for UUID/path resolution)

## Goals

- Replace fragile name-based links with UUID-based links that survive renames
- Provide autocomplete for both `[[` (wikilink-style trigger) and `[text](cerbo://` (URL-style trigger)
- Integrate UUID resolution seamlessly into rendering pipeline
- Leverage existing `cerbo resolve` mechanism for path lookup

## Non-Goals

- Migrating existing `[[Page Name]]` links to new format (separate follow-up task)
- Supporting other URL schemes (only cerbo://)
- Real-time bi-directional link graphs (rendering-only in this change)
- Auto-updating stale link text when pages are renamed

## Technical Decisions

### Decision 1: Autocomplete Trigger Points
**Two entry points for link insertion:**
- Typing `[[` → inserts `[Page Name](cerbo://objects/<uuid>)` (wikilink-style UX, new format)
- Typing `[text](cerbo://` or even "(c.." → autocomplete shows vault objects to complete the URL

**Why:**
- Wikilinks are familiar to users, so maintain that entry point but modernize the output
- Cerbo:// entry point enables users who know UUID or prefer exploring via URL scheme
- Both approaches converge on the same link format

**Alternatives considered:**
- Only support cerbo:// scheme (rejected: loses familiar wikilink UX)
- Auto-convert old wikilinks on edit (rejected: too aggressive, separate migration task)

### Decision 2: UUID Resolution via Click Handler
**Render cerbo:// links as-is with semantic href; resolve UUID on click**

Process:
1. Parse markdown for links with `cerbo://objects/<uuid>` pattern
2. Render as `<a href="cerbo://objects/<uuid>">Link Text</a>` (semantic href)
3. Attach click handler to resolve UUID to current path using `cerbo resolve`
4. On successful resolution, navigate to resolved path; on error, show user feedback

**Why:**
- Preserves semantic link format in href (transparent to user)
- Lazy resolution (only on click, not during render)
- Avoids blocking render pipeline on UUID lookups
- Better UX for SPA/desktop app (client-side navigation)
- UUID resolution failure is handled gracefully at interaction time

**Alternatives considered:**
- Server-side render-time resolution (rejected: blocks render, no lazy loading)
- Store path in link alongside UUID (rejected: maintains stale data, defeats purpose)
- Build in-memory UUID index (rejected: complexity, must sync with vault)

### Decision 3: Autocomplete Data Structure
**Fetch full vault object list with type, title, and UUID for autocomplete**

In editor:
1. On editor init or vault change, load object list (minimal: `{uuid, title, type}`)
2. Store in component state or cache
3. Filter/search as user types
4. Display as `[Type] Title` in autocomplete menu

**Why:**
- Fast client-side filtering (no round-trip per keystroke)
- Can show type to disambiguate objects with similar titles
- Minimal data transfer (only what's needed for autocomplete)

**Alternatives considered:**
- Server-side search API (rejected: adds latency, unnecessary complexity)
- Only load objects on first `[[` trigger (rejected: user expects instant response)

### Decision 4: Link Text Handling
**Link text in `[...]` is user-controlled and not auto-updated**

- User types `[[Task List` → autocomplete inserts `[Task List](cerbo://objects/<uuid>)`
- If page later renamed to "All Tasks", link text stays `[Task List]` (stale but intentional)
- Navigation still works because href uses UUID
- If user wants up-to-date text, they manually edit source or tooling can offer "refresh link text" (future)

**Why:**
- Users may intentionally write custom link text different from title
- Avoids unintended side effects of auto-updating (user loses control)
- Simpler implementation (no link text reconciliation logic)

**Alternatives considered:**
- Auto-update link text on every render (rejected: surprises users, loses intentional customization)
- Track which links are "auto-generated" vs "custom" (rejected: complexity)

### Decision 5: Backward Compatibility
**During transition, support both wikilink and cerbo:// formats in rendering**

Render pipeline:
- `[[Page Name]]` → try to resolve by name (old behavior, warning in logs if fragile)
- `[text](cerbo://objects/<uuid>)` → resolve via UUID (new behavior, canonical)
- Old links continue to work, new links use new format

Autocomplete:
- Only inserts new cerbo:// format
- Editing an old wikilink doesn't auto-convert (stays as-is until user re-edits)

**Why:**
- Allows gradual rollout without breaking existing vaults
- Users can incrementally update links as they edit pages
- Separate migration task can handle bulk conversion if needed

**Alternatives considered:**
- Force conversion of all old links on load (rejected: destructive, loses user intent)
- Only support new format from day 1 (rejected: breaks existing vaults)

## Risks & Trade-offs

| Risk | Mitigation |
|------|-----------|
| **UUID not found on click** — User clicks cerbo:// link but UUID doesn't exist | Show error dialog; offer fallback (e.g., search, go to vault root) |
| **Stale link text confuses users** — User sees `[Old Title]` linking to renamed page | Document behavior; consider future "refresh link text" tooling; warning on hover if text differs from actual title |
| **Click handler not attached** — Link renders but handler fails to initialize | Test in both desktop and web; fallback to default link behavior if handler missing |
| **Vault with many objects** — Autocomplete object list grows large (1000+) | Lazy-load or pagination; server-side filtering if needed in future |
| **Cross-vault links** — User clicks cerbo:// URL from different vault | Scope resolution to current vault; show error if UUID not in active vault |

## Migration Plan

### Phase 1: Feature Implementation (this change)
- Add cerbo:// autocomplete to editor
- Update wikilink autocomplete to insert cerbo:// format
- Update renderer to resolve cerbo:// links
- Maintain old wikilink rendering for backward compatibility
- Tests verify both formats work

### Phase 2: Link Migration (future task)
- Bulk conversion tool: scan all vault pages, convert `[[Name]]` → `[Name](cerbo://objects/<uuid>)`
- Optional: user can run migration or do manually over time

### Phase 3: Deprecation (future, if decided)
- After users have time to migrate, consider deprecating `[[...]]` format
- Remove old wikilink rendering, log error for any remaining old links
- Requires separate RFC/decision

## Open Questions

1. **Vault listing performance**: How large can the vault object list grow before autocomplete lags? Do we need pagination or server-side search?
2. **Cerbo resolve API**: Is there a batch resolve endpoint, or must we call once per UUID? Should we add one?
3. **Link text sync**: In the future, should we offer a "refresh all link text to match current titles" tool? Or leave it manual?
4. **Cross-vault links**: Should users be able to link to objects in other vaults (e.g., `cerbo://vault-id/objects/<uuid>`)? Or keep it single-vault for now?
5. **Permission model**: If rendered pages are shared/exported, do cerbo:// links resolve the same way for other users/contexts?

## Implementation Notes

- **Editor**: SvelteKit Svelte component with markdown editor + autocomplete
- **Rendering**: Markdown-it or similar parser hook for cerbo:// links
- **Resolution**: Call into core's `cerbo resolve` (or new batch endpoint)
- **Testing**: Unit tests for autocomplete filtering; integration tests for render+resolve; E2E tests for user flow
