## MODIFIED Requirements

### Requirement: Read page
The system SHALL read the raw markdown content of a page from its `page.md` file and return it to the frontend. The frontend SHALL render the page title from the first level-1 heading (`# `) within the content in the preview mode.

#### Scenario: Read existing page
- **WHEN** the frontend requests the content of a page by slug
- **THEN** the system returns the raw markdown string from `<slug>/page.md`
- **THEN** the frontend renders the markdown preview with the title included as the first heading

## ADDED Requirements

### Requirement: Unified Editor Interface
The system SHALL provide a unified editor interface using a tabbed layout (e.g., "Write" and "Preview") to switch between editing and viewing modes.

#### Scenario: Switching to preview mode
- **WHEN** the user selects the "Preview" tab in the editor
- **THEN** the system SHALL render the markdown content as HTML, including the page title
- **THEN** the system SHALL attach interactive handlers to wikilinks within the preview
