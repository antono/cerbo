## ADDED Requirements

### Requirement: Render task list items without bullets in preview
The system SHALL render markdown task list items in page preview without list bullets while preserving the list item's text layout.

#### Scenario: Render checked and unchecked task items
- **WHEN** the page preview renders markdown containing task list items
- **THEN** the task list items SHALL be displayed without bullets
- **AND** the checkbox and text SHALL remain aligned on the same line

### Requirement: Render larger task list checkboxes in preview
The system SHALL render task list checkboxes at a larger visual size in preview without changing the line height of the surrounding content.

#### Scenario: Display enlarged checkbox
- **WHEN** the page preview renders a task list item
- **THEN** the checkbox SHALL be visually larger than the default inline checkbox
- **AND** the line height of the list item SHALL remain unchanged

### Requirement: Toggle task list checkbox state in preview
The system SHALL allow the user to toggle a task list checkbox by clicking it in page preview.

#### Scenario: Check an unchecked task
- **WHEN** the user clicks an unchecked task list checkbox in preview
- **THEN** the checkbox SHALL become checked
- **AND** the rendered task state SHALL update immediately in the preview

#### Scenario: Uncheck a checked task
- **WHEN** the user clicks a checked task list checkbox in preview
- **THEN** the checkbox SHALL become unchecked
- **AND** the rendered task state SHALL update immediately in the preview

### Requirement: Persist task list checkbox state to markdown source
The system SHALL persist task list checkbox state changes back to the page's markdown source.

#### Scenario: Save checkbox toggle to source
- **WHEN** the user toggles a task list checkbox in preview
- **THEN** the underlying markdown content SHALL be updated to reflect the new checked or unchecked state
- **AND** the page SHALL be saved using the existing markdown write path
