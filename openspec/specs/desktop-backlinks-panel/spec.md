# desktop-backlinks-panel Specification

## Purpose
TBD - created by archiving change adapt-cerbo-desktop-to-new-storage-model. Update Purpose after archive.
## Requirements
### Requirement: BacklinkEntry carries uuid and title
The frontend type `BacklinkEntry` SHALL have the shape `{ uuid: string; title: string }`. The field `slug` SHALL NOT exist. The Tauri `backlinks_get` command SHALL return `Vec<BacklinkEntry>` (resolved), not `Vec<String>` (raw UUIDs).

#### Scenario: backlinks_get returns resolved entries
- **WHEN** the frontend calls `invoke('backlinks_get', { uuid })`
- **THEN** each entry in the returned array SHALL have a `uuid` field
- **THEN** each entry SHALL have a `title` field read from the source page's `meta.ttl`
- **THEN** no entry SHALL have a `slug` field
- **THEN** entries whose source object has no readable `meta.ttl` SHALL be omitted silently

#### Scenario: Empty backlinks
- **WHEN** no other page links to the current page
- **THEN** `backlinks_get` SHALL return an empty array
- **THEN** the panel SHALL display an empty-state message

### Requirement: backlinks_get resolves UUIDs in the Tauri command layer
The `backlinks_get` Tauri command SHALL read raw UUIDs from `backrefs.ttl` via `cerbo_core::links::backrefs_read`, then resolve each UUID to `{ uuid, title }` by reading `meta.ttl` for each source object. Resolution SHALL happen in the Rust command layer, not in the frontend.

#### Scenario: Resolution reads meta.ttl for each backlink
- **WHEN** `backrefs_read` returns `[uuid-a, uuid-b]` for the current page
- **THEN** the command reads `.cerbo/objects/uuid-a/meta.ttl` and extracts `:title`
- **THEN** the command reads `.cerbo/objects/uuid-b/meta.ttl` and extracts `:title`
- **THEN** the command returns `[{ uuid: "uuid-a", title: "Page A" }, { uuid: "uuid-b", title: "Page B" }]`

#### Scenario: Missing meta.ttl is skipped
- **WHEN** a backref UUID points to an object directory that lacks `meta.ttl`
- **THEN** that entry SHALL be silently omitted from the returned list
- **THEN** the command SHALL NOT return an error

### Requirement: Backlinks panel navigates by uuid
The frontend backlinks panel SHALL navigate to a source page by calling `openPage(entry.uuid)`. It SHALL NOT use `entry.slug`.

#### Scenario: User clicks a backlink entry
- **WHEN** the user clicks a `BacklinkEntry` in the backlinks panel
- **THEN** `openPage(entry.uuid)` is called
- **THEN** the editor loads the content of the source page identified by that UUID
- **THEN** the backlinks panel updates to show backlinks for the newly opened page

