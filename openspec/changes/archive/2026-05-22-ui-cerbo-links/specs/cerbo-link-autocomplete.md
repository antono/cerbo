# Spec: Cerbo Link Autocomplete

When user types `[text](cerbo://`, trigger autocomplete showing all vault objects with their type and title visible in the completion list.

## ADDED Requirements

### Requirement: Cerbo URL scheme autocomplete
When user begins typing a cerbo:// link, the editor SHALL display autocomplete showing available objects in the vault with their type and title.

#### Scenario: Typing cerbo:// triggers autocomplete
- **WHEN** user types `[Some Text](cerbo://` (or `(cerbo://`)
- **THEN** autocomplete menu appears showing available vault objects
- **AND** autocomplete is not dismissed until user selects an item or explicitly closes it

#### Scenario: Autocomplete list displays type and title
- **WHEN** autocomplete is active for cerbo:// links
- **THEN** each entry shows: `[Type] Page Name` or similar format
- **AND** user can see object type (e.g., "Page", "Note", "Task") and title to distinguish objects
- **AND** the full cerbo:// URL is visible (e.g., as a subtext or on hover)

#### Scenario: User selects object from cerbo:// autocomplete
- **WHEN** user confirms selection from cerbo:// autocomplete
- **THEN** the full cerbo:// URL (e.g., `cerbo://objects/<uuid>`) is inserted
- **AND** link text in brackets is preserved as originally typed

#### Scenario: Filtering cerbo:// autocomplete by type or text
- **WHEN** user types `[text](cerbo://Note` or similar text after the scheme
- **THEN** autocomplete filters to show matching objects (by type prefix or title substring)
- **AND** user can further refine selection

#### Scenario: Empty vault
- **WHEN** vault contains no objects
- **THEN** autocomplete appears but shows empty state or "No objects found"
- **AND** user can dismiss autocomplete and continue editing

### Requirement: Cerbo URL format correctness
All autocomplete insertions SHALL produce valid `cerbo://objects/<uuid>` URLs that can be resolved.

#### Scenario: UUID format in completion
- **WHEN** user selects an object from autocomplete
- **THEN** the UUID inserted is valid and matches the object's actual UUID in the vault
- **AND** format is exactly `cerbo://objects/<uuid>` with no variations

#### Scenario: Non-existent UUID
- **WHEN** user manually types a cerbo:// URL with a UUID that doesn't exist
- **THEN** link is preserved in source (not rejected)
- **AND** renders as broken or unresolved during render phase
