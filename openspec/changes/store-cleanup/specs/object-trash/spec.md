# Object Trash

## Purpose
Define recoverable deletion for vault objects. Deleting a page SHALL move its files aside
rather than destroy them, so that an accidental or mistaken deletion can be recovered from
the vault itself without a backup.

## ADDED Requirements

### Requirement: Delete moves objects to trash
Deleting an object SHALL relocate its directory into a trash area inside the vault rather
than removing its contents. The object SHALL no longer appear in page listings, link
resolution, or the materialised symlink tree.

#### Scenario: Delete a page
- **WHEN** the user deletes the page with UUID `<uuid>`
- **THEN** `.cerbo/objects/<uuid>/` SHALL no longer exist
- **THEN** the object's files SHALL exist under `.cerbo/trash/`
- **THEN** `page.md`, `meta.ttl`, and any other files in the object SHALL be preserved byte-for-byte

#### Scenario: Deleted page disappears from listings
- **WHEN** a page has been deleted
- **THEN** it SHALL NOT appear in the page list
- **THEN** it SHALL NOT appear in the materialised symlink tree after the next rebuild

#### Scenario: Deleted page no longer resolves
- **WHEN** a page has been deleted and its UUID is resolved
- **THEN** resolution SHALL report the object as not found

### Requirement: Trash entries are uniquely named and identifiable
Each trash entry SHALL be named so that it identifies the deleted object and does not
collide with any previous or future deletion of the same object.

#### Scenario: Same object deleted, recreated, and deleted again
- **WHEN** an object is deleted, an object with the same UUID is later created, and it is deleted again
- **THEN** both deletions SHALL exist as separate trash entries
- **THEN** neither entry SHALL overwrite the other

#### Scenario: Trash entry identifies its object
- **WHEN** the user inspects a trash entry
- **THEN** the entry name SHALL contain the deleted object's UUID
- **THEN** the entry name SHALL indicate when the deletion occurred

### Requirement: Trash is excluded from all vault operations
The trash area SHALL be treated as outside the live object set. Indexing, page listing, link
resolution, backreference maintenance, and symlink materialisation SHALL ignore it.

#### Scenario: Reindex ignores trash
- **WHEN** a full vault reindex runs and the trash contains deleted objects
- **THEN** no metadata or backreferences SHALL be derived from trash contents
- **THEN** the reindex SHALL NOT report trash entries as problems

#### Scenario: Symlink rebuild ignores trash
- **WHEN** the symlink tree is rebuilt and the trash is non-empty
- **THEN** no symlink SHALL be created for any trashed object
- **THEN** the rebuild SHALL NOT treat the trash directory as unsafe content

#### Scenario: Links to a trashed object are broken, not resurrecting
- **WHEN** a live page still contains a link to a trashed object and that page is saved
- **THEN** the trashed object SHALL NOT be recreated under `.cerbo/objects/`
- **THEN** the link SHALL be reported as broken

### Requirement: Read-only objects cannot be deleted
Deletion SHALL continue to be refused for objects whose type is read-only, and the refusal
SHALL happen before anything is moved.

#### Scenario: Delete a source-type object
- **WHEN** the user attempts to delete an object whose type is read-only
- **THEN** the operation SHALL fail with an error stating the object is read-only
- **THEN** the object SHALL remain in `.cerbo/objects/` unchanged
- **THEN** no trash entry SHALL be created

### Requirement: Trash retention is user-controlled
The system SHALL NOT delete trash entries automatically. Trash SHALL be removable only by
explicit user action, and the vault SHALL remain fully functional while trash is present.

#### Scenario: Trash is never auto-pruned
- **WHEN** the vault is used normally over many sessions with trash present
- **THEN** no trash entry SHALL be removed without an explicit user request

#### Scenario: Trash is excluded from version control by default
- **WHEN** a vault is initialised
- **THEN** the vault's ignore rules SHALL exclude the trash area from version control
