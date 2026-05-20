# Spec: Dynamic Slug Generation

## ADDED Requirements

### Requirement: Slug auto-generation from page name
The system SHALL automatically generate a URL-friendly slug from the page name as the user types, transforming it to lowercase, replacing spaces with hyphens, and removing invalid characters.

#### Scenario: User types page name
- **WHEN** user enters "My New Page" in the page name field
- **THEN** slug field is auto-populated with "my-new-page"

#### Scenario: Slug updates as page name changes
- **WHEN** user updates page name from "My Page" to "My New Page"
- **THEN** slug updates from "my-page" to "my-new-page"

#### Scenario: Slug generation handles special characters
- **WHEN** user enters "Page (v2) & Notes!" as page name
- **THEN** slug becomes "page-v2-notes" (parentheses, ampersand removed)

### Requirement: Manual slug override disables auto-update
The system SHALL disable auto-generation once the user manually edits the slug field. Subsequent changes to the page name SHALL NOT update the slug.

#### Scenario: User manually edits slug
- **WHEN** user edits the slug field directly
- **THEN** auto-update flag is disabled for this dialog session

#### Scenario: Page name change after manual slug edit
- **WHEN** slug has been manually edited and user changes page name
- **THEN** slug remains unchanged (respects user override)

### Requirement: Slug validation
The system SHALL validate that slug contains only alphanumeric characters, hyphens, and underscores.

#### Scenario: Valid slug format
- **WHEN** slug is "my-page_v2"
- **THEN** validation passes

#### Scenario: Invalid slug format
- **WHEN** slug contains spaces or special characters like "my page!" or "my@page"
- **THEN** validation fails and form submit is blocked with error message

#### Scenario: Empty slug validation
- **WHEN** user attempts to submit with empty slug
- **THEN** validation fails with error "Slug is required"

### Requirement: Dialog state per form instance
The system SHALL maintain auto-update state (enabled/disabled) separately for each new-page dialog instance.

#### Scenario: Multiple dialogs with independent state
- **WHEN** user opens two new-page dialogs simultaneously
- **THEN** manual slug edit in one dialog does not affect auto-update in the other dialog
