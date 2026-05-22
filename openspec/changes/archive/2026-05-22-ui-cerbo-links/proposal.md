# Proposal: Replace Wikilinks with Cerbo UUID-based Links

## Summary
Replace wikilink format (`[[Page Name]]`) with cerbo's native UUID-based links (`[Page Name](cerbo://objects/<uuid>)`). This provides deterministic linking that survives page renames and leverages the vault's UUID storage model.

## Why This Matters
Wikilinks are fragile—they break when pages are renamed. By switching to cerbo's UUID-based link format (`cerbo://objects/<uuid>`), links become rename-proof and align with the vault's UUID storage architecture. During rendering, these links are resolved to current page paths via the `cerbo resolve` command.

## What Changes

### Current Behavior
- User types `[[` → autocomplete of page names
- Confirmed selection inserts `[[Page Name]]`
- Renders as HTML links to cerbo pages

### New Behavior
- User types `[[` → autocomplete shows page names with UUIDs
- Confirmed selection inserts `[Page Name](cerbo://objects/<uuid>)`
- User can also type `[text](cerbo://` → autocomplete shows all vault objects with type and title
- During rendering, cerbo:// links are resolved to full paths using `cerbo resolve`

## Capabilities

### New
- **Wikilink to Cerbo Link Insertion**: When autocomplete is confirmed after typing `[[`, insert the cerbo:// format link with UUID instead of wikilink syntax.
- **Cerbo Link Autocomplete**: When user types `[something](cerbo://`, trigger autocomplete showing all cerbo:// URLs in the vault with object type and title visible.
- **Cerbo Link Resolution**: During the rendering phase, resolve `cerbo://objects/<uuid>` links to their current page paths using the cerbo resolve mechanism.

### Modified
- **Wikilink Autocomplete**: Existing wikilink autocomplete behavior now inserts cerbo:// format links instead of `[[...]]` syntax.

## Impact
- **Source Files**: Vault markdown files will use `[Page Name](cerbo://objects/<uuid>)` format instead of `[[Page Name]]`
- **Editor**: Autocomplete in both wikilink and cerbo:// link contexts
- **Rendering**: Link resolution pipeline must handle cerbo:// scheme
- **API/Commands**: Uses existing `cerbo resolve` mechanism
- **Migrations**: Existing `[[Page Name]]` wikilinks should be migrated to cerbo:// format (separate task)

## Success Criteria
- [ ] Typing `[[` triggers autocomplete with page names and UUIDs
- [ ] Confirming autocomplete inserts `[Page Name](cerbo://objects/<uuid>)`
- [ ] Typing `[text](cerbo://` triggers autocomplete showing all vault objects with type/title
- [ ] Links render correctly with proper href resolution
- [ ] Cerbo:// links remain functional if source page is renamed
- [ ] No console errors or broken links in rendered output

## Questions / Assumptions
- Should existing `[[Page Name]]` links be automatically migrated? (Suggest: yes, as a follow-up task)
- Should cerbo:// be the only supported link format for internal links, or support both? (Suggest: transition fully to cerbo://)
