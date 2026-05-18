# Backlinks — Delta Spec

Change: adapt-cerbo-desktop-to-new-storage-model

## MODIFIED Requirements

### Requirement: Display backlinks panel
The system SHALL display a backlinks panel for the currently open page, listing all pages in the active vault that link to the current page. The panel SHALL reside within a multi-section right sidebar. Each entry SHALL be identified by `uuid` and `title`; there SHALL be no `slug` field. Each entry SHALL be navigable by UUID.

#### Scenario: Page with backlinks
- **WHEN** the user opens a page that is linked from other pages
- **THEN** the backlinks panel lists each linking page by title
- **THEN** each entry is clickable and calls `openPage(entry.uuid)`
- **THEN** no slug is used to identify or navigate to the source page

#### Scenario: Page with no backlinks
- **WHEN** the user opens a page that no other page links to
- **THEN** the backlinks panel displays an empty state message

### Requirement: Backlink computation
The system SHALL read backlinks from `.cerbo/objects/<uuid>/backrefs.ttl` using the `cerbo_core::links::backrefs_read` function. The Tauri `backlinks_get` command SHALL resolve each raw UUID to `{ uuid, title }` by reading the source object's `meta.ttl` before returning to the frontend. The system SHALL NOT return raw `Vec<String>` to the frontend.

#### Scenario: backlinks_get returns resolved BacklinkEntry list
- **WHEN** the user views backlinks for a page with UUID `<uuid-target>`
- **THEN** the Tauri command reads `.cerbo/objects/<uuid-target>/backrefs.ttl`
- **THEN** it resolves each source UUID to `{ uuid, title }` via `ObjectMeta::read_from_file`
- **THEN** it returns `Vec<BacklinkEntry>` where each entry has `uuid` and `title`
- **THEN** the return type SHALL NOT be `Vec<String>`

#### Scenario: Get backlinks for a page — internal flow
- **WHEN** `backrefs_read(ctx, uuid-target)` returns `[uuid-a, uuid-b]`
- **THEN** `meta.ttl` is read for `uuid-a` and `uuid-b`
- **THEN** the command returns `[{ uuid: "uuid-a", title: "..." }, { uuid: "uuid-b", title: "..." }]`

#### Scenario: Backrefs.ttl contains only backlinks
- **WHEN** reading `<uuid-target>/backrefs.ttl`
- **THEN** it SHALL contain ONLY `:hasBacklink` triples
- **THEN** it SHALL NOT contain outgoing `:linksTo` or `:usesAttachment`
