## MODIFIED Requirements

### Requirement: Rename page
The system SHALL rename a page by: (1) deriving a new slug from the new title, (2) renaming the page folder, (3) identifying and updating all `[[OldTitle]]` occurrences using the link index, (4) rebuilding the link index.

#### Scenario: Rename page with no incoming links
- **WHEN** the user renames a page that no other page links to
- **THEN** the folder is renamed to the new slug
- **THEN** no other files are modified

#### Scenario: Rename page with incoming links
- **WHEN** the user renames "Rust Ownership" to "Rust Memory Model"
- **THEN** the folder `rust-ownership/` is renamed to `rust-memory-model/`
- **THEN** the system identifies only pages linking to "Rust Ownership" using the link index
- **THEN** only those pages are modified, replacing `[[Rust Ownership]]` with `[[Rust Memory Model]]`
- **THEN** the link index is rebuilt

#### Scenario: Rename to a conflicting slug
- **WHEN** the user renames a page to a title whose slug already exists as another page
- **THEN** the system SHALL reject the operation with a descriptive error
- **THEN** no files are modified
