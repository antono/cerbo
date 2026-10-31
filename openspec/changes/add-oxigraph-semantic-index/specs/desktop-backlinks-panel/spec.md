# Spec Delta

## MODIFIED Requirements

### Requirement: BacklinkEntry carries uuid and title
The frontend type `BacklinkEntry` SHALL have the shape `{ uuid: string; title: string }`. The field `slug` SHALL NOT exist. The Tauri `backlinks_get` command SHALL return `Vec<BacklinkEntry>` (resolved), not `Vec<String>` (raw UUIDs).

#### Scenario: backlinks_get returns resolved entries
- **WHEN** the frontend calls `invoke('backlinks_get', { uuid })`
- **THEN** each entry in the returned array SHALL have a `uuid` field
- **THEN** each entry SHALL have a `title` field taken from the source page's metadata in the semantic index
- **THEN** no entry SHALL have a `slug` field
- **THEN** entries whose source object has no title in the index SHALL be omitted silently

#### Scenario: Empty backlinks
- **WHEN** no other page links to the current page
- **THEN** `backlinks_get` SHALL return an empty array
- **THEN** the panel SHALL display an empty-state message

## REMOVED Requirements

### Requirement: backlinks_get resolves UUIDs in the Tauri command layer
**Reason**: There are no raw backlink UUIDs to resolve. One core index query returns UUIDs and titles together.
**Migration**: See "backlinks_get queries the semantic index".

## ADDED Requirements

### Requirement: backlinks_get queries the semantic index

The `backlinks_get` Tauri command SHALL return the backlinks defined by the `backlinks` capability, obtained from the vault's semantic index through `cerbo-core`. It SHALL NOT read `backrefs.ttl` or any per-source `meta.ttl`. When the index is busy or cannot be opened, it SHALL return an error that the panel shows as a non-blocking message instead of the list.

#### Scenario: Backlinks after another device synced a page
- **WHEN** a page linking to the current page arrived through file sync
- **THEN** the panel SHALL list it the next time backlinks are loaded

#### Scenario: Index busy
- **WHEN** the CLI holds the index longer than the retry limit while the user opens a page
- **THEN** the panel SHALL show that backlinks are temporarily unavailable
- **THEN** the page itself SHALL open and be editable

#### Scenario: First open after upgrade
- **WHEN** the user opens a vault that has no index yet
- **THEN** the index SHALL be built and the panel SHALL show a loading state until it is ready
