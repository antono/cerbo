# Spec: Virtual Path Input

## ADDED Requirements

### Requirement: Virtual path text input
The system SHALL provide a text input field for the virtual path (mount path) that accepts paths in "path/name" format.

#### Scenario: User enters valid path
- **WHEN** user enters "docs/guides" in the virtual path field
- **THEN** input is accepted and stored

#### Scenario: Single segment path
- **WHEN** user enters "root" (single path segment)
- **THEN** input is accepted

#### Scenario: Deep path hierarchy
- **WHEN** user enters "docs/guides/tutorials/getting-started"
- **THEN** input is accepted with multiple levels

### Requirement: Virtual path autocomplete
The system SHALL provide autocomplete suggestions based on existing paths in the vault as the user types, showing all paths that match the prefix (case-insensitive).

#### Scenario: Autocomplete shows matching paths
- **WHEN** vault contains paths ["docs/guides", "docs/api", "docs/examples"] and user types "docs/g"
- **THEN** autocomplete list shows ["docs/guides"]

#### Scenario: Autocomplete is case-insensitive
- **WHEN** vault contains "Docs/Guides" and user types "docs/g"
- **THEN** autocomplete matches and suggests "Docs/Guides"

#### Scenario: No matches in autocomplete
- **WHEN** user types "xyz/" which has no matching paths in the vault
- **THEN** autocomplete list is empty

#### Scenario: User selects from autocomplete
- **WHEN** autocomplete list is shown and user clicks a suggestion
- **THEN** virtual path field is filled with the selected path

#### Scenario: Autocomplete updates on keystroke
- **WHEN** user types "d" → vault shows ["docs/*"], then user continues typing "ocs/g"
- **THEN** autocomplete list updates dynamically with each keystroke showing only "docs/guides"

### Requirement: Virtual path validation
The system SHALL validate that virtual path contains only valid path characters (alphanumeric, hyphens, underscores, forward slashes).

#### Scenario: Valid path characters
- **WHEN** path is "docs/my-guide_v2/section"
- **THEN** validation passes

#### Scenario: Invalid path characters
- **WHEN** path contains special characters like "docs/my@guide" or "docs\\backslash"
- **THEN** validation fails and form submit is blocked with error message

#### Scenario: Empty path validation
- **WHEN** user attempts to submit with empty virtual path
- **THEN** validation fails with error "Virtual path is required"

#### Scenario: Path with trailing slash
- **WHEN** path is "docs/guides/"
- **THEN** validation passes (trailing slash is allowed)

### Requirement: Path loading from vault context
The system SHALL load existing paths from the vault to populate the autocomplete list when the dialog opens.

#### Scenario: Autocomplete list populated on dialog open
- **WHEN** new-page dialog is opened
- **THEN** existing paths from the vault are fetched and available for autocomplete suggestions

#### Scenario: Empty vault with no paths
- **WHEN** vault has no existing paths and user types in virtual path field
- **THEN** autocomplete list remains empty (no suggestions available)
