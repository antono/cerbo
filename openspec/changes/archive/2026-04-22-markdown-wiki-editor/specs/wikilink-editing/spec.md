## ADDED Requirements

### Requirement: Wikilink syntax highlighting
The system SHALL render `[[wikilink]]` syntax distinctly in the Carta editor — resolved links in one style, broken links in another.

#### Scenario: Render resolved wikilink
- **WHEN** the editor contains `[[Rust Ownership]]` and `rust-ownership/page.md` exists
- **THEN** the link is rendered as a clickable inline element with a resolved style

#### Scenario: Render broken wikilink
- **WHEN** the editor contains `[[Nonexistent Page]]` and no matching slug exists
- **THEN** the link is rendered with a broken/unresolved style (e.g., muted or struck)

### Requirement: Wikilink navigation
The system SHALL navigate to the target page when the user clicks a resolved wikilink in the editor or preview.

#### Scenario: Click resolved wikilink
- **WHEN** the user clicks a resolved `[[wikilink]]`
- **THEN** the application navigates to the target page

#### Scenario: Click broken wikilink
- **WHEN** the user clicks a broken `[[wikilink]]`
- **THEN** the application offers to create a new page with that title

### Requirement: Wikilink autocomplete
The system SHALL provide autocomplete suggestions when the user types `[[` in the editor, filtering page titles as the user continues typing.

#### Scenario: Trigger autocomplete
- **WHEN** the user types `[[` in the editor
- **THEN** a dropdown appears listing existing pages in the active vault

#### Scenario: Filter autocomplete results
- **WHEN** the user types `[[rust`
- **THEN** the dropdown filters to pages whose title or slug contains "rust"

#### Scenario: Select autocomplete suggestion
- **WHEN** the user selects a suggestion from the dropdown
- **THEN** the editor inserts the full `[[Page Title]]` wikilink and closes the dropdown
