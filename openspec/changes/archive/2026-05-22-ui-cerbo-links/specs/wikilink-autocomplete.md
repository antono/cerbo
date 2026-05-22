# Spec: Wikilink Autocomplete (Modified)

Existing wikilink autocomplete behavior is modified to display UUID information and support the new cerbo:// insertion format.

## MODIFIED Requirements

### Requirement: Autocomplete displays UUID information
The wikilink autocomplete (`[[...`) menu SHALL display each page with both its name and UUID to help users identify objects.

#### Scenario: UUID shown alongside page name
- **WHEN** user types `[[` to trigger autocomplete
- **THEN** autocomplete list displays each page with: Name and UUID (e.g., `"My Page (abc-123-def)"` or with separate columns)
- **AND** UUID display is compact and readable
- **AND** user can identify the correct page even if multiple pages share similar names

#### Scenario: Search/filter by name still works
- **WHEN** user types `[[Task` to filter autocomplete
- **THEN** autocomplete shows pages matching "Task" in their title
- **AND** each match displays with its UUID
- **AND** filtering works the same as before, just with UUID added

#### Scenario: Autocomplete sorting
- **WHEN** user triggers autocomplete
- **THEN** results are sorted in a consistent order (e.g., by title, by recency, by relevance)
- **AND** sorting is predictable and user can rely on order
- **AND** if user types partial text, closest matches appear first

### Requirement: Integration with new cerbo:// format
The wikilink autocomplete SHALL integrate with the cerbo:// insertion mechanism so selecting from `[[` autocomplete produces cerbo:// links.

#### Scenario: Autocomplete selection produces correct format
- **WHEN** user types `[[` and selects a page from autocomplete
- **THEN** the inserted link is in cerbo:// format: `[Page Name](cerbo://objects/<uuid>)`
- **AND** NOT in old wikilink format: `[[Page Name]]`

#### Scenario: Transition period (if both formats supported)
- **WHEN** both wikilink and cerbo:// formats are supported in rendering
- **THEN** autocomplete always inserts cerbo:// format (new canonical format)
- **AND** old `[[...]]` links in vault files still render correctly
- **AND** future editing of old links converts them to new format on save (optional future behavior)
