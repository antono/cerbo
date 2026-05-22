# Spec: Wikilink to Cerbo Link Insertion

When user confirms autocomplete after typing `[[`, insert cerbo:// format link with the page's UUID instead of wikilink syntax.

## ADDED Requirements

### Requirement: Autocomplete produces cerbo:// link format
When a user triggers link insertion via `[[` autocomplete, the editor SHALL insert `[Page Name](cerbo://objects/<uuid>)` format instead of `[[Page Name]]` syntax.

#### Scenario: User completes wikilink autocomplete for existing page
- **WHEN** user types `[[`, selects a page from autocomplete, and confirms
- **THEN** editor inserts `[Page Name](cerbo://objects/<uuid>)` at cursor position
- **AND** `<uuid>` is the correct UUID of the selected page

#### Scenario: Cursor position after insertion
- **WHEN** user completes wikilink autocomplete insertion
- **THEN** cursor is positioned after the closing `)`
- **AND** user can immediately continue typing or start a new link

#### Scenario: Link text matches page title
- **WHEN** user confirms autocomplete for a page
- **THEN** the link text portion `[Page Name]` matches the page's current title exactly
- **AND** link remains valid if page is later renamed (UUID doesn't change)

#### Scenario: Invalid or deleted page
- **WHEN** user attempts to insert a link for a page that no longer exists
- **THEN** autocomplete does not offer that page as an option
- **AND** if manually typed with a non-existent UUID, link renders as broken during render phase

## MODIFIED Requirements

### Requirement: Wikilink autocomplete
The existing wikilink autocomplete behavior is modified to support the new cerbo:// insertion format and display UUID alongside page names for clarity.

#### Scenario: Autocomplete list shows UUIDs
- **WHEN** user types `[[` to trigger autocomplete
- **THEN** autocomplete list displays each page with its name and UUID (or compact UUID display)
- **AND** user can still identify pages by name

#### Scenario: Multi-page matches
- **WHEN** user types `[[Task` and multiple pages match (e.g., "Task List", "Tasks Overview")
- **THEN** autocomplete shows all matching pages with names and UUIDs
- **AND** user can select the correct one to insert the matching link
