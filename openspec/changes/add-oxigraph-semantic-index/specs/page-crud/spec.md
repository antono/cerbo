# Spec Delta

## REMOVED Requirements

### Requirement: Write page
**Reason**: The write path no longer updates `backrefs.ttl` on target objects.
**Migration**: Replaced by "Write page content". Backlinks come from the semantic index.

## ADDED Requirements

### Requirement: Write page content

The system SHALL write updated markdown content to a page's `page.md` atomically, update its own `meta.ttl` and `annotations.ttl`, and update the page's triples in the semantic index. It SHALL NOT write any other object's files. The system SHALL NOT write to `type: :Source` pages (read-only).

#### Scenario: Save page content
- **WHEN** the frontend provides a UUID and updated markdown content via `page_write`
- **THEN** the content is written to `.cerbo/objects/<uuid>/page.md`
- **THEN** `meta.ttl` `schema:dateModified` is updated
- **THEN** the page's outgoing `cerbo://` links are recorded in the semantic index
- **THEN** no file outside `.cerbo/objects/<uuid>/` is modified

#### Scenario: Write to source type (read-only)
- **WHEN** the user attempts to write to a page with `type: :Source` in `meta.ttl`
- **THEN** the system SHALL return an error "Cannot write to source type (read-only)"
- **THEN** no changes are written to disk

#### Scenario: Link removed on save
- **WHEN** a page previously linked to `cerbo://<uuid-b>` and the user saves without that link
- **THEN** B's backlinks SHALL NOT include the page
- **THEN** no file of object B SHALL be modified

## MODIFIED Requirements

### Requirement: Delete page
The system SHALL delete a page by relocating its entire `.cerbo/objects/<uuid>/` directory
into the vault's trash area, as specified by the `object-trash` capability. The system SHALL
NOT delete `type: :Source` pages. The operation MUST require confirmation via a modal dialog.
The page's files SHALL be recoverable from the trash; the operation SHALL NOT destroy content.

#### Scenario: Delete existing page
- **WHEN** the user triggers a delete operation for a page with `type: :Page`
- **THEN** the system SHALL display a confirmation modal showing the page title
- **WHEN** the user confirms the deletion
- **THEN** the directory `.cerbo/objects/<uuid>/` SHALL no longer exist
- **THEN** the page's files SHALL exist under `.cerbo/trash/` with their content preserved
- **THEN** the page's triples, including its outgoing links, SHALL be removed from the semantic index
- **THEN** no other object's files SHALL be written

#### Scenario: Delete source type (read-only)
- **WHEN** the user attempts to delete a page with `type: :Source`
- **THEN** the system SHALL return an error "Cannot delete source type (read-only)"
- **THEN** no files or folders are removed or relocated

#### Scenario: Deleted page is not resurrected by a later save
- **WHEN** a page has been deleted and another page still linking to it is saved
- **THEN** the deleted object's directory SHALL NOT be recreated under `.cerbo/objects/`
