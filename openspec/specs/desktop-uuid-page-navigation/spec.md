# desktop-uuid-page-navigation Specification

## Purpose
TBD - created by archiving change adapt-cerbo-desktop-to-new-storage-model. Update Purpose after archive.
## Requirements
### Requirement: PageMeta carries uuid not slug
The frontend type `PageMeta` SHALL have the shape `{ uuid: string; title: string }`. The field `slug` SHALL NOT exist. The Tauri `page_list` command SHALL return this shape. All frontend code that previously keyed on `slug` SHALL key on `uuid`.

#### Scenario: page_list returns uuid entries
- **WHEN** the frontend calls `invoke('page_list')`
- **THEN** each entry in the returned array SHALL have a `uuid` field containing a UUID v4 string
- **THEN** each entry SHALL have a `title` field
- **THEN** no entry SHALL have a `slug` field

#### Scenario: Frontend finds page by uuid
- **WHEN** the frontend looks up a page in the `app.pages` array
- **THEN** it SHALL use `pages.find(p => p.uuid === targetUuid)`
- **THEN** it SHALL NOT use `p.slug`

### Requirement: App state tracks current page by uuid
The reactive app state SHALL use `currentUuid` (not `currentSlug`) to track the open page. The `history` array SHALL store UUID strings. The transient dialog state fields SHALL be `renameUuid` and `confirmDeleteUuid`.

#### Scenario: Opening a page sets currentUuid
- **WHEN** the user opens a page
- **THEN** `app.currentUuid` SHALL be set to that page's UUID
- **THEN** `app.history` SHALL contain the UUID at the current history index

#### Scenario: Delete dialog references uuid
- **WHEN** the user triggers a delete confirmation
- **THEN** `app.confirmDeleteUuid` SHALL be set to the target page UUID
- **THEN** the confirmation dialog SHALL look up the page title via `pages.find(p => p.uuid === app.confirmDeleteUuid)`

#### Scenario: Rename dialog references uuid
- **WHEN** the user triggers a rename operation
- **THEN** `app.renameUuid` SHALL be set to the target page UUID
- **THEN** the rename dialog SHALL look up the current title via `pages.find(p => p.uuid === app.renameUuid)`

### Requirement: Tauri invoke calls pass uuid
All Tauri `invoke()` calls that previously passed a `slug` parameter SHALL pass `uuid` instead. The Tauri commands `page_read`, `page_write`, `page_delete`, `attachment_list`, `attachment_add`, `attachment_delete`, `vault_update_last_page`, `backlinks_get` SHALL accept `uuid` as the page identifier parameter. The commands `page_create` and `page_list` SHALL NOT accept a `vaultId` parameter.

#### Scenario: page_read invoked with uuid
- **WHEN** the frontend calls `invoke('page_read', { uuid })`
- **THEN** the command succeeds and returns the page content
- **THEN** no `slug` or `vaultId` parameter is passed

#### Scenario: page_write invoked with uuid
- **WHEN** the frontend calls `invoke('page_write', { uuid, content })`
- **THEN** the command writes the content and returns it
- **THEN** backlinks are updated (see backlinks spec)

#### Scenario: vault_update_last_page stores uuid
- **WHEN** the user opens a page
- **THEN** the frontend calls `invoke('vault_update_last_page', { vaultId, uuid })`
- **THEN** the state.toml stores the UUID string as `last_open_page` for that vault

### Requirement: Vault open restores last page by uuid
On vault open the system SHALL attempt to restore the previously open page by matching `lastOpenPage` against `page.uuid` values. If no match is found (stale slug, null, or missing page) the system SHALL fall back gracefully without error.

#### Scenario: Restore by uuid match
- **WHEN** the vault is opened and `lastOpenPage` is a UUID present in `app.pages`
- **THEN** that page SHALL be opened automatically

#### Scenario: Stale session value does not match
- **WHEN** `lastOpenPage` contains a slug string (legacy) or a UUID that no longer exists
- **THEN** `app.pages.find(p => p.uuid === last)` returns undefined
- **THEN** the system SHALL fall back to opening a page titled "Index" if one exists, otherwise the first page in the list
- **THEN** no error SHALL be shown to the user

#### Scenario: Empty vault after stale restore fails
- **WHEN** `lastOpenPage` does not match and no pages exist
- **THEN** `currentUuid` SHALL be null and the editor SHALL show an empty state

### Requirement: page_update_title command
The desktop SHALL expose a `page_update_title(uuid, newTitle)` Tauri command that updates the `:title` in `meta.ttl` and the H1 heading in `page.md` for the given UUID. This replaces the former slug-based `page_rename` command.

#### Scenario: Rename updates meta and heading
- **WHEN** the user submits a new title "Advanced Rust" for a page via the rename dialog
- **THEN** the frontend calls `invoke('page_update_title', { uuid, newTitle: 'Advanced Rust' })`
- **THEN** `meta.ttl` `:title` is updated to "Advanced Rust"
- **THEN** the first H1 heading in `page.md` is updated to `# Advanced Rust`
- **THEN** `app.pages` entry for this UUID reflects the new title after `page_list` reload

#### Scenario: page_rename command does not exist
- **WHEN** any caller attempts `invoke('page_rename', ...)`
- **THEN** the command SHALL NOT be registered and the call SHALL fail
- **THEN** callers MUST use `page_update_title` instead

### Requirement: Remove slug_from_title command
The desktop SHALL NOT register the `slug_from_title` Tauri command. The concept of a page slug does not exist in the desktop navigation layer.

#### Scenario: slug_from_title is not invocable
- **WHEN** any frontend code calls `invoke('slug_from_title', ...)`
- **THEN** the call SHALL fail with a command-not-found error
- **THEN** no slug preview SHALL be shown in the new-page or rename dialogs

