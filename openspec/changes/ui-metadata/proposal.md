# Proposal: UI Metadata – Custom Slug and Virtual Path

## Summary
Add custom `slug` and `virtualPath` (mount path) fields to the new page dialog with smart defaults and user-controlled updates.

## Goals
- **User convenience**: Auto-generate slug from page name as user types, but allow manual override when needed
- **Path management**: Enable users to organize pages in virtual mount paths with autocomplete
- **Predictable behavior**: Stop auto-updating slug once user manually changes it (preserve user intent)

## New Capabilities

### Dynamic Slug Generation
- `slug` field is prefilled and updated in real-time as user types the page name
- Once user manually edits the slug, auto-update is disabled for that dialog session
- Slug validation: only alphanumeric, hyphens, underscores (standard URL-friendly format)

### Virtual Path Input
- `virtualPath` (mount path) field accepts `path/name` format
- Autocomplete suggestions based on existing paths in the vault
- Simple text input (no picker UI at this stage)

### State Management
- Track "slug auto-update enabled" flag per dialog instance
- Clear flag when user makes first edit to slug field

## New Behavior
- New page dialog form structure:
  - Page name (existing)
  - Slug (new, auto-populated)
  - Virtual path / mount path (new, with autocomplete)
- Form submission validates all fields before creating page

## Existing Capabilities Unchanged
- No changes to page creation API
- No changes to existing page metadata storage
- No changes to vault structure

## Impact
- **UI**: New form component enhancements (slug field with auto-update logic, autocomplete path input)
- **State**: Dialog state includes slug auto-update flag
- **Data**: Slug and virtualPath passed to page creation API

## Unlocks
- Design: UI component specifications and form layout
- Specs: Input behavior, validation rules, autocomplete algorithm
