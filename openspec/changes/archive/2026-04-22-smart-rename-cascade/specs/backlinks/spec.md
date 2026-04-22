## MODIFIED Requirements

### Requirement: Build link index
The system SHALL build a link index for the active vault by parsing all `page.md` files and extracting `[[wikilink]]` occurrences. The index SHALL be stored as a cache at `$XDG_CACHE_HOME/cerbo/<vault-id>/index.json` and SHALL serve as the authoritative source for discovering page relationships for both UI and maintenance operations (e.g., rename cascade).

#### Scenario: Build index on vault open (cache missing)
- **WHEN** a vault is opened and no cache file exists
- **THEN** the system scans all `*/page.md` files and builds the index
- **THEN** the index is written to the cache path

#### Scenario: Build index on vault open (cache stale)
- **WHEN** a vault is opened and the cache `builtAt` timestamp is older than the vault's most recent file modification time
- **THEN** the system rebuilds the index from scratch

#### Scenario: Incremental update on page save
- **WHEN** a `page.md` file is modified
- **THEN** the system updates only that page's entry in the index
- **THEN** the backlinks panel refreshes automatically
