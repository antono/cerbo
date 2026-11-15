# Page Attachments

## Purpose
Enable users to manage file attachments as separate UUID-based objects.

## Requirements

### Requirement: Attachment as object
The system SHALL store an attachment as its own object in `.cerbo/objects/<uuid>/`: the file itself plus a `meta.ttl` that types the object `schema:MediaObject` and records the MIME type in `schema:encodingFormat`. The object SHALL have no `page.md`. A file named `meta.ttl` or `page.md` SHALL be stored as `attachment-<name>` and keep its own name as the title. Adding an attachment SHALL NOT change any page: a page uses an attachment only by linking to it.

#### Scenario: Add an attachment
- **WHEN** the file `/path/to/file.png` is added as an attachment
- **THEN** a new UUID is generated for the attachment
- **THEN** the file is copied to `.cerbo/objects/<uuid-attachment>/file.png` and the source is kept
- **THEN** `meta.ttl` types the object `schema:MediaObject` with `schema:encodingFormat "image/png"`
- **THEN** no page file is written

#### Scenario: Attachment storage layout
- **WHEN** listing contents of `.cerbo/objects/<uuid-attachment>/`
- **THEN** it SHALL contain the binary file (e.g., `image.png`)
- **THEN** it SHALL contain `meta.ttl` with attachment metadata
- **THEN** it SHALL NOT contain `page.md`

#### Scenario: A failed copy leaves nothing behind
- **WHEN** copying the file into the vault fails
- **THEN** no object directory for the attachment remains

### Requirement: List page attachments
The system SHALL list the attachments a page uses by reading the `cerbo://<uuid>` links in the page's `page.md` and keeping those whose target is an existing Attachment object, each once, in order of first appearance. No other file records which attachments a page uses.

#### Scenario: Request attachment list for page
- **WHEN** a page links to the attachment A twice and then to the attachment B
- **THEN** the list is A, B

#### Scenario: Links to pages are not attachments
- **WHEN** a page links to another page and to an attachment
- **THEN** the list holds only the attachment

#### Scenario: A link to a deleted attachment is skipped
- **WHEN** a page links to an attachment that no longer exists
- **THEN** the list does not include it

### Requirement: Delete page attachment
The system SHALL allow users to delete an attachment object. Deleting moves `.cerbo/objects/<uuid>/` to the vault trash, like any other object. Pages that link to the attachment SHALL NOT be edited.

#### Scenario: Deleting an attachment
- **WHEN** the attachment `<uuid-attachment>` is deleted
- **THEN** `.cerbo/objects/<uuid-attachment>/` no longer exists
- **THEN** its files are in `.cerbo/trash/`
- **THEN** the pages that linked to it keep their text, and the attachment no longer shows in their attachment lists

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
