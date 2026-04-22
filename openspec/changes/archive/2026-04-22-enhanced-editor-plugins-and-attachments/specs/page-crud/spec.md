## MODIFIED Requirements

### Requirement: Unified Editor Interface
The system SHALL provide a unified editor interface using a tabbed layout (e.g., "Write" and "Preview") to switch between editing and viewing modes. The editor SHALL support a standard set of plugins for enhanced productivity.

#### Scenario: Switching to preview mode
- **WHEN** the user selects the "Preview" tab in the editor
- **THEN** the system SHALL render the markdown content as HTML, including the page title
- **THEN** the system SHALL attach interactive handlers to wikilinks within the preview

#### Scenario: Using enhanced editor features
- **WHEN** the user types markdown in the "Write" tab
- **THEN** the system SHALL provide syntax highlighting for code blocks
- **THEN** the system SHALL support emoji shortcodes and a file attachment picker
