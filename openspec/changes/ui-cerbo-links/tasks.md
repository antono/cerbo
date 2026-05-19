# Tasks: Replace Wikilinks with Cerbo UUID-based Links

## 1. Editor Preparation & Object Listing

- [ ] 1.1 Create vault object listing utility (core or CLI) to fetch `{uuid, title, type}` for all objects in vault
- [ ] 1.2 Add vault object list endpoint or export to editor context (API or direct core call)
- [ ] 1.3 Store vault object list in editor state/store with vault change detection (re-fetch on vault switch)
- [ ] 1.4 Add unit tests for object list fetching and state management

## 2. Wikilink Autocomplete Enhancement

- [ ] 2.1 Modify `[[` trigger autocomplete to fetch and display UUID alongside each page name
- [ ] 2.2 Update autocomplete list UI to show format `[Type] Page Name` with UUID visible (tooltip or secondary text)
- [ ] 2.3 Implement client-side search/filtering for autocomplete (filter by title or partial match)
- [ ] 2.4 Add sorting to autocomplete results (by title, recency, or relevance)
- [ ] 2.5 Update selection handler to insert `[Page Name](cerbo://objects/<uuid>)` instead of `[[Page Name]]`
- [ ] 2.6 Test autocomplete flow end-to-end with actual vault data

## 3. Cerbo Link Autocomplete (New)

- [ ] 3.1 Add parser hook for `[text](cerbo://` to detect cerbo link intent in editor
- [ ] 3.2 Implement cerbo:// autocomplete trigger logic (show vault objects when user types scheme)
- [ ] 3.3 Update autocomplete menu to show `[Type] Page Name` with full cerbo:// URL visible
- [ ] 3.4 Implement selection handler to complete cerbo:// URL with `cerbo://objects/<uuid>`
- [ ] 3.5 Test that user can manually type `[text](cerbo://` and get autocomplete
- [ ] 3.6 Add edge case handling (empty vault, non-existent UUID, special characters in text)

## 4. Rendering: Cerbo Link Click Handler

- [ ] 4.1 Add regex pattern matcher for `[text](cerbo://objects/<uuid>)` in markdown renderer
- [ ] 4.2 Render cerbo:// links with semantic href: `<a href="cerbo://objects/<uuid>">text</a>` (no transformation)
- [ ] 4.3 Implement click handler that calls `cerbo resolve <uuid>` on link click
- [ ] 4.4 Handle resolution errors gracefully in click handler (non-existent UUID → show error dialog)
- [ ] 4.5 Trigger navigation to resolved path after successful resolution
- [ ] 4.6 Test link rendering and click behavior with valid and invalid UUIDs
- [ ] 4.7 Test link click behavior when linked page is renamed (should resolve and navigate to new path)
- [ ] 4.8 Test click handler attachment in both desktop (Tauri) and web contexts

## 5. Backward Compatibility: Wikilink Rendering

- [ ] 5.1 Verify existing `[[Page Name]]` wikilink rendering still works (old behavior)
- [ ] 5.2 Ensure old wikilinks resolve by name matching (may be fragile, but functional during transition)
- [ ] 5.3 Add optional logging/warning if old wikilink format is used (for future deprecation)
- [ ] 5.4 Test that both formats coexist in the same document without conflicts

## 6. Integration & Testing

- [ ] 6.1 Create integration test: type `[[`, select page, verify cerbo:// link is inserted
- [ ] 6.2 Create integration test: type `[text](cerbo://`, select object, verify URL is completed
- [ ] 6.3 Create integration test: render page with cerbo:// link, verify href resolves correctly
- [ ] 6.4 Create integration test: rename linked page, render, verify link still works (UUID resolution)
- [ ] 6.5 Test both desktop (Tauri) and web editor autocomplete and rendering
- [ ] 6.6 Test with large vault (100+ objects) to ensure no UI lag in autocomplete
- [ ] 6.7 Test cerbo:// links across different vaults (should scope correctly or error appropriately)

## 7. Documentation & QA

- [ ] 7.1 Update user docs: explain new cerbo:// link format and how to use both `[[` and cerbo:// entry points
- [ ] 7.2 Document UUID resolution behavior: renames don't break links, link text may be stale
- [ ] 7.3 Add docs or tip about refreshing link text if desired (manual for now)
- [ ] 7.4 QA smoke test: create test vault, add pages, create links via both methods, verify rendering
- [ ] 7.5 QA test: rename page, verify incoming links still navigate (resolve to new path)
- [ ] 7.6 Verify no console errors or warnings in browser/Tauri dev tools
- [ ] 7.7 Test with existing vaults: old wikilinks should still render

## 8. Future / Out of Scope (Post-Implementation)

- [ ] 8.1 Migration tool to bulk-convert `[[Name]]` → `[Name](cerbo://objects/<uuid>)` (separate task)
- [ ] 8.2 "Refresh all link text" tool to sync stale link text with current page titles
- [ ] 8.3 Batch UUID resolution API in core if `cerbo resolve` becomes a bottleneck
- [ ] 8.4 Cross-vault linking support if needed
- [ ] 8.5 Deprecation of old `[[...]]` format (future RFC)

## Implementation Order

1. Start with object listing (tasks 1.x) — all other features depend on it
2. Implement wikilink autocomplete changes (tasks 2.x) — familiar UX, quick win
3. Add cerbo:// autocomplete (tasks 3.x) — new feature, builds on same object list
4. Build renderer support (tasks 4.x) — can be tested independently
5. Verify backward compatibility (tasks 5.x) — ensures no regressions
6. Comprehensive testing (tasks 6.x) — integration tests cover all flows
7. Documentation and QA (tasks 7.x) — final polish and smoke tests
