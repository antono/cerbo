## MODIFIED Requirements

### Requirement: Similar notes section in the right sidebar

The Links sub-panel of the right panel SHALL show a "Similar notes" section below Backlinks and Outgoing links. For the open page it SHALL list up to 10 other pages closest in meaning, best first, each with its title and similarity score as a percentage. Clicking an entry SHALL open that page by UUID. The open page SHALL never appear in its own list.

#### Scenario: Related notes are listed
- **WHEN** the user opens a page about a trip to Berlin, the vault has embeddings and Links is selected
- **THEN** the section SHALL list pages closest in meaning with titles and scores
- **THEN** clicking an entry SHALL call `openPage(entry.uuid)`

#### Scenario: Section updates on navigation
- **WHEN** the user opens another page
- **THEN** the section SHALL show the similar notes of the newly opened page

#### Scenario: Not loaded while hidden
- **WHEN** another tab is selected
- **THEN** similar notes SHALL NOT be computed until Links is selected
