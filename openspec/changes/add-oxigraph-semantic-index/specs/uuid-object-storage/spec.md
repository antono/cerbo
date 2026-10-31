# Spec Delta

## MODIFIED Requirements

### Requirement: Object Storage Layout
The system SHALL store all objects (pages, attachments, ontologies) under
`.cerbo/objects/<uuid>/` directory within the vault root. Each object MUST have a `meta.ttl`
file. Pages and ontologies MUST have `page.md`. Attachments store their binary file. Objects
MAY have `annotations.ttl`. Objects SHALL NOT have a `backrefs.ttl`; backlinks are derived
data held only in the semantic index.

An object's class SHALL be exactly one of four schema.org classes, which also decides its
behaviour: `schema:NoteDigitalDocument` for a user-created page (formerly `:Product`),
`schema:WebPage` for imported read-only content (formerly `:Source`), `schema:MediaObject`
for a binary file (formerly `:Attachment`), and `schema:DefinedTermSet` for an imported
ontology (formerly `:Ontology`). In the rest of the specifications, "Product", "Source",
"Attachment" and "Ontology" name these four classes.

#### Scenario: Create new page object
- **WHEN** user runs `cerbo create "My Page"`
- **THEN** a new UUID is generated (v4)
- **THEN** directory `.cerbo/objects/<uuid>/` is created
- **THEN** `page.md` is created with `# My Page` as content
- **THEN** `meta.ttl` is created recording the class `schema:NoteDigitalDocument`

#### Scenario: Object directory structure
- **WHEN** listing `.cerbo/objects/<uuid>/` contents
- **THEN** `meta.ttl` MUST exist
- **THEN** `page.md` exists for Product/Source/Ontology types
- **THEN** binary file exists for Attachment type
- **THEN** `backrefs.ttl` SHALL NOT exist, however many objects link to this one
- **THEN** `annotations.ttl` exists if object has HackMD annotations
