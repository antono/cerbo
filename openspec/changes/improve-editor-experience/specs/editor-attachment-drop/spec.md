# Spec Delta

## Purpose

Lets the user drop or paste files and fragments into the page editor so they
become Attachment objects of the current vault, attached to the current page and
referenced at the drop point.

## ADDED Requirements

### Requirement: Dropped file becomes an Attachment object
When a file from the operating system is dropped on the editor in write mode,
the system SHALL copy it into the current vault as a new `Attachment` object
(file and `meta.ttl` with title and MIME type) and SHALL leave the source file
untouched.

#### Scenario: Drop a PDF
- **WHEN** the user drops `report.pdf` on the editor
- **THEN** a new object `.cerbo/objects/<uuid>/` holds `report.pdf` and a `meta.ttl` with type `Attachment` and MIME type `application/pdf`
- **AND** the original `report.pdf` is still in its source folder

#### Scenario: Drop several files
- **WHEN** the user drops three files at once
- **THEN** three Attachment objects are created, in the order the files were dropped

### Requirement: Dropped fragment becomes an Attachment object
When a non-file drag (an image, HTML or text selection from another window or
application) is dropped on the editor, the system SHALL store its content as a
new `Attachment` object: image data as an image file, HTML as `.html`, plain
text as `.txt`, named after the source when known.

#### Scenario: Drop an image from a browser
- **WHEN** the user drags an image out of a web page onto the editor
- **THEN** an Attachment object holding that image file is created

#### Scenario: Drop a text selection
- **WHEN** the user drags selected plain text from another application onto the editor
- **THEN** an Attachment object holding a `.txt` file with that text is created

### Requirement: Reference inserted at the drop point
After the Attachment is stored, the editor SHALL insert a reference at the drop
point: `![<name>](cerbo://objects/<uuid>)` for images, `[<name>](cerbo://objects/<uuid>)`
otherwise. The reference SHALL make the Attachment appear in the page's attachments.

#### Scenario: Image is shown as an image
- **WHEN** the user drops `photo.png`
- **THEN** `![photo.png](cerbo://objects/<uuid>)` is inserted where it was dropped
- **AND** the preview renders the image

#### Scenario: Other files are links
- **WHEN** the user drops `report.pdf`
- **THEN** `[report.pdf](cerbo://objects/<uuid>)` is inserted where it was dropped

#### Scenario: Attachment listed on the page
- **WHEN** the dropped reference has been saved
- **THEN** the attachments panel of the current page lists the new Attachment

### Requirement: Pasted file becomes an Attachment object
Pasting into the editor in write mode a clipboard that holds a file (copied in a
file manager) or image data (e.g. a screenshot) SHALL create an `Attachment`
object and insert its reference at the caret, exactly as a drop does. Pasting
plain text SHALL keep inserting the text as before.

#### Scenario: Paste a file copied in the file manager
- **WHEN** the user copies `report.pdf` in the file manager and presses Ctrl+V in the editor
- **THEN** an Attachment object holding `report.pdf` is created
- **AND** `[report.pdf](cerbo://objects/<uuid>)` is inserted at the caret
- **AND** the attachments panel of the current page lists it after save

#### Scenario: Paste a screenshot
- **WHEN** the clipboard holds image data and the user presses Ctrl+V
- **THEN** an Attachment object holding a PNG image is created
- **AND** `![<name>](cerbo://objects/<uuid>)` is inserted at the caret

#### Scenario: Paste an image copied in another program
- **WHEN** the user picks "Copy image" in a browser, image viewer or editor and presses Ctrl+V in the editor
- **THEN** an Attachment object holding that image is created and `![<name>](cerbo://objects/<uuid>)` is inserted at the caret
- **AND** the image is used even when the clipboard also offers HTML or text for it

#### Scenario: Paste plain text
- **WHEN** the clipboard holds only text
- **THEN** the text is inserted as text and no Attachment is created

### Requirement: Drop feedback and failure
The editor SHALL show a drop indicator while something is dragged over it, and
SHALL report a failed copy without inserting a reference.

#### Scenario: Drag over the editor
- **WHEN** the user drags a file over the editor
- **THEN** the editor shows a drop indicator at the would-be insertion point

#### Scenario: Copy fails
- **WHEN** the file cannot be read or the vault is not writable
- **THEN** an error naming the file is shown
- **AND** no reference is inserted and no partial object is left in the vault
