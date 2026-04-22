## MODIFIED Requirements

### Requirement: Display backlinks panel
The system SHALL display a backlinks panel for the currently open page, listing all pages in the active vault that contain a wikilink resolving to the current page. The panel SHALL reside within a multi-section right sidebar.

#### Scenario: Page with backlinks
- **WHEN** the user opens a page that is linked from other pages
- **THEN** the backlinks panel lists each linking page by title
- **THEN** each entry is clickable and navigates to that page

#### Scenario: Page with no backlinks
- **WHEN** the user opens a page that no other page links to
- **THEN** the backlinks panel displays an empty state message
