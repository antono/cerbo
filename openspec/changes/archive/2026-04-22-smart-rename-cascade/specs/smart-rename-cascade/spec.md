## ADDED Requirements

### Requirement: Index-driven link discovery
The system SHALL identify pages containing links to a target page by querying the link index instead of scanning all files on disk.

#### Scenario: Find linking pages via index
- **WHEN** renaming a page with slug `old-slug` and title `Old Title`
- **THEN** the system retrieves all `PageEntry` objects from the index
- **THEN** it filters for entries where `links` contains "Old Title" (case-insensitive) or "old-slug"
- **THEN** it returns only the slugs of the identified pages for targeted rewriting
