# Backlinks

## Purpose
Enable users to discover relationships between pages by listing all incoming links for the current page, using cached `backrefs.ttl` files instead of index-based computation.
## Requirements

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

### Requirement: Update backlinks on page save
The system SHALL update `backrefs.ttl` for TARGET objects when a page's links change. The
source page does NOT store outgoing links in `backrefs.ttl` (they're in `page.md`). Only
`:hasBacklink` in target objects is updated. The link difference SHALL be computed and
applied exactly once per save. A backreference SHALL only be written to an object that
already exists; the system SHALL NOT create an object, or any part of one, as a side effect
of recording a backreference.

#### Scenario: Page adds link to another page
- **WHEN** user saves `page-a` with a new link to `cerbo://<uuid-b>`
- **THEN** `uuid-b/backrefs.ttl` SHALL contain `:hasBacklink <cerbo://objects/<uuid-a>>`
- **THEN** `page-a` does NOT store outgoing links in `backrefs.ttl` (just in `page.md`)

#### Scenario: Page removes link to another page
- **WHEN** user saves `page-a` and the link to `cerbo://<uuid-b>` is removed
- **THEN** `uuid-b/backrefs.ttl` SHALL NOT contain `:hasBacklink <cerbo://objects/<uuid-a>>`

#### Scenario: Link difference is applied once per save
- **WHEN** a page is saved with one added link
- **THEN** the target's `backrefs.ttl` SHALL be updated exactly once
- **THEN** the same difference SHALL NOT be recomputed and reapplied by a second code path

#### Scenario: Link to a non-existent object
- **WHEN** user saves a page containing `[B](cerbo://objects/deadbeef-0000-0000-0000-000000000137)` and no such object exists
- **THEN** no directory SHALL be created under `.cerbo/objects/`
- **THEN** no `backrefs.ttl` SHALL be created for that UUID
- **THEN** the save SHALL succeed and the link SHALL be reported as broken

#### Scenario: Link to a deleted object does not resurrect it
- **WHEN** object `C` has been deleted and a page still linking to `C` is saved
- **THEN** `.cerbo/objects/C/` SHALL NOT be recreated
- **THEN** `C` SHALL NOT become resolvable

#### Scenario: Backreference failure is reported
- **WHEN** a backreference update fails while saving a page
- **THEN** the save SHALL report an error rather than returning success

### Requirement: backrefs.ttl structure
The system SHALL store ONLY backlinks (incoming links from OTHER objects) in `backrefs.ttl`
using Turtle RDF syntax. The file SHALL contain ONLY `:hasBacklink` predicates. Outgoing
links are stored in `page.md` as `cerbo://<uuid>` (no tracking file needed). The file SHALL
be a syntactically valid Turtle document whose subject is the owning object's own IRI, and it
SHALL be written atomically.

#### Scenario: backrefs.ttl contains only backlinks
- **WHEN** reading `backrefs.ttl` for an object
- **THEN** it SHALL contain ONLY `:hasBacklink` triples
- **THEN** it SHALL NOT contain `:linksTo` or `:usesAttachment` (those are in `page.md`)
- **THEN** each `:hasBacklink` represents another object that links TO this object

#### Scenario: How backlinks get cached
- **WHEN** Page A saves `page.md` with link `[Page B](cerbo://<uuid-b>)`
- **THEN** `<uuid-b>/backrefs.ttl` SHALL be updated with `:hasBacklink <cerbo://objects/<uuid-a>>`
- **THEN** Page A does NOT store outgoing links in any `.ttl` file (just in `page.md`)

#### Scenario: backrefs.ttl parses with a conformant Turtle parser
- **WHEN** any object's `backrefs.ttl` is parsed by a conformant Turtle parser
- **THEN** parsing SHALL succeed with no syntax errors
- **THEN** the subject SHALL be the owning object's `<cerbo://objects/<uuid>>` IRI

#### Scenario: Object with no backlinks
- **WHEN** an object has no incoming links
- **THEN** its `backrefs.ttl` SHALL be a valid Turtle document asserting no `:hasBacklink`
- **THEN** reading it SHALL yield an empty backlink list
