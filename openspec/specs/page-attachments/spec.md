# Page Attachments

## Purpose
Enable users to manage file attachments as separate UUID-based objects.

## Requirements

### Requirement: Attachment as object
The system SHALL store attachments as separate objects in `.cerbo/objects/<uuid>/` with `meta.ttl` containing `type: :Attachment` and `:mime-type`. The attachment binary file SHALL be stored directly in the object directory.

#### Scenario: Add attachment to page
- **WHEN** user runs `cerbo page attachment add <vault-id> <page-uuid> /path/to/file.png`
- **THEN** a new UUID is generated for the attachment
- **THEN** `.cerbo/objects/<uuid-attachment>/` directory is created
- **THEN** the file is copied to `.cerbo/objects/<uuid-attachment>/file.png`
- **THEN** `meta.ttl` is created with `type: :Attachment` and `:mime-type "image/png"`
- **THEN** the page's `relations.ttl` is updated with `:usesAttachment <uuid-attachment>`

#### Scenario: Attachment storage layout
- **WHEN** listing contents of `.cerbo/objects/<uuid-attachment>/`
- **THEN** it SHALL contain the binary file (e.g., `image.png`)
- **THEN** it SHALL contain `meta.ttl` with attachment metadata
- **THEN** it SHALL NOT contain `page.md`

### Requirement: List page attachments
The system SHALL return a list of attachment UUIDs used by a page by reading the page's `relations.ttl` for `:usesAttachment` predicates.

#### Scenario: Request attachment list for page
- **WHEN** the user opens the attachments panel for a page
- **THEN** the system reads the page's `relations.ttl`
- **THEN** it returns all UUIDs with `:usesAttachment` predicate
- **THEN** each attachment's title and mime-type are read from its `meta.ttl`

### Requirement: Delete page attachment
The system SHALL allow users to delete an attachment object. This removes the `.cerbo/objects/<uuid>/` directory.

#### Scenario: Deleting an attachment
- **WHEN** the user clicks "Delete" on an attachment
- **THEN** the system SHALL remove `.cerbo/objects/<uuid-attachment>/` directory
- **THEN** the page's `relations.ttl` is updated to remove the `:usesAttachment` reference

### Requirement: Render attachment in page
The system SHALL support `![Alt Text](cerbo://<uuid>)` markdown syntax for embedding attachments in pages.

#### Scenario: Embed image attachment
- **WHEN** `page.md` contains `![Screenshot](cerbo://<uuid>)`
- **THEN** the editor/preview SHALL resolve the UUID to the attachment's file path
- **THEN** the image SHALL be rendered in the preview

#### Scenario: Link to non-image attachment
- **WHEN** `page.md` contains `[Download PDF](cerbo://<uuid>)`
- **THEN** the editor/preview SHALL resolve to the PDF file path
- **THEN** a download link SHALL be rendered
