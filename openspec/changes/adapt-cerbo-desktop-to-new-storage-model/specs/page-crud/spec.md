# Page CRUD — Delta Spec

Change: adapt-cerbo-desktop-to-new-storage-model

## MODIFIED Requirements

### Requirement: Write page
The system SHALL write updated markdown content to a page's `page.md` file atomically via `cerbo_core::links::page_write_with_links`. The system SHALL update `backrefs.ttl` on all TARGET objects whose link status changes as a result of this write. The system SHALL NOT write to `type: :Source` pages (read-only).

#### Scenario: Save page content updates backrefs
- **WHEN** the frontend provides a UUID and updated markdown content via `page_write`
- **THEN** the Tauri command calls `page_write_with_links(ctx, uuid, content)`
- **THEN** the content is written to `.cerbo/objects/<uuid>/page.md`
- **THEN** outgoing `cerbo://` links are extracted from the new content
- **THEN** `backrefs.ttl` is updated on each target object to reflect added or removed links
- **THEN** `meta.ttl` `schema:dateModified` is updated

#### Scenario: Write to source type (read-only)
- **WHEN** the user attempts to write to a page with `type: :Source` in `meta.ttl`
- **THEN** the system SHALL return an error "Cannot write to source type (read-only)"
- **THEN** no changes are written to disk

#### Scenario: Link removed on save clears backref
- **WHEN** a page previously linked to `cerbo://<uuid-b>` and the user saves without that link
- **THEN** `page_write_with_links` detects the removed link
- **THEN** `.cerbo/objects/<uuid-b>/backrefs.ttl` SHALL NOT contain the source page's UUID

### Requirement: Rename page
The system SHALL rename a page by updating its `:title` in `meta.ttl` and its H1 heading in `page.md` using the `page_update_title(uuid, newTitle)` Tauri command. The page's UUID and directory location SHALL NOT change. The UI SHALL provide a focused modal dialog showing the current title. The former slug-based `page_rename` command SHALL NOT be used.

#### Scenario: Rename page via page_update_title
- **WHEN** the user provides a new title "Advanced Rust" for a page via the rename dialog
- **THEN** `page_update_title` updates `:title` in `meta.ttl` to "Advanced Rust"
- **THEN** the first H1 heading in `page.md` is updated to `# Advanced Rust`
- **THEN** the UUID and directory location SHALL NOT change
- **THEN** `page_list` subsequently returns the updated title for this UUID

#### Scenario: page_rename command is not registered
- **WHEN** any caller invokes `page_rename`
- **THEN** the call SHALL fail — the command is not registered in the Tauri invoke handler

### Requirement: List pages
The system SHALL return a list of all pages by scanning `.cerbo/objects/` directories that contain `page.md`. Each entry SHALL include `uuid` and `title`. The command SHALL NOT accept a `vaultId` parameter — page storage is global within the app context.

#### Scenario: page_list returns uuid entries
- **WHEN** the frontend calls `invoke('page_list')` without a `vaultId`
- **THEN** the system returns one entry per object with `page.md` and a valid page type
- **THEN** each entry includes `uuid` (UUID v4 string) and `title` (from `meta.ttl`)
- **THEN** no entry includes a `slug` field

#### Scenario: page_list ignores vaultId parameter
- **WHEN** `page_list` is invoked with any extra parameter
- **THEN** the extra parameter is ignored
- **THEN** all pages in the configured objects directory are returned

## REMOVED Requirements

### Requirement: slug_from_title desktop command
**Reason**: The slug concept does not exist in desktop page navigation. Pages are identified by UUID. The `slug_from_title` Tauri command and its `src-tauri/src/slug.rs` module are dead code.
**Migration**: No replacement needed in the desktop. Slug derivation remains in `cerbo_core::slug` for use by the CLI symlink and index subsystems only.
