# Page Metadata Index

## MODIFIED Requirements

### Requirement: Extract and regenerate backref metadata

The system SHALL parse each `page.md` file for `cerbo://<uuid>` links and update the
corresponding `backrefs.ttl` files for each referenced page. A full-vault reindex SHALL
accumulate the complete backreference set in a single pass over the vault and write each
affected object's `backrefs.ttl` at most once. It SHALL NOT clear every object's
backreferences and then rebuild them, so that an interruption cannot leave the vault with
backreferences erased.

#### Scenario: Page with outgoing links
- **WHEN** page A contains `cerbo://uuid-b` and `cerbo://uuid-c`
- **THEN** system adds page A's UUID to `backrefs.ttl` in both page B and page C's object directories

#### Scenario: Page with no links
- **WHEN** page contains no `cerbo://` links
- **THEN** no backref files are modified for other pages (but this page's backrefs may still be updated by other pages linking to it)

#### Scenario: Broken link reference
- **WHEN** page contains `cerbo://nonexistent-uuid`
- **THEN** system logs a warning but continues processing (orphaned link)
- **THEN** no object directory or `backrefs.ttl` SHALL be created for that UUID

#### Scenario: Each affected object is written once per reindex
- **WHEN** a full reindex runs over a vault where 137 pages all link to the same hub page
- **THEN** the hub's `backrefs.ttl` SHALL be written once, containing all 137 backreferences
- **THEN** it SHALL NOT be rewritten once per incoming link

#### Scenario: Interrupted reindex does not erase backreferences
- **WHEN** a full reindex is interrupted partway through
- **THEN** every object's `backrefs.ttl` SHALL contain either its previous or its newly computed content
- **THEN** no object SHALL be left with its backreferences cleared

#### Scenario: Reindex converges on a stale backreference
- **WHEN** page A no longer links to page B but B's `backrefs.ttl` still records A
- **AND** a full-vault reindex runs
- **THEN** B's `backrefs.ttl` SHALL NOT record A afterwards

#### Scenario: Reindex reports problems through its exit status
- **WHEN** a reindex encounters an object it cannot read or write
- **THEN** the problem SHALL be reported to stderr with the offending UUID
- **THEN** the command SHALL exit non-zero
