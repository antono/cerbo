# UUID Object Storage

## Purpose
Define the core UUID-based object storage model for Cerbo vaults, replacing slug-based page storage with `.cerbo/objects/<uuid>/` layout.

## ADDED Requirements

### Requirement: Object Storage Layout
The system SHALL store all objects (pages, attachments, ontologies) under `.cerbo/objects/<uuid>/` directory within the vault root. Each object MUST have a `meta.ttl` file. Pages and ontologies MUST have `page.md`. Attachments store their binary file. Objects MAY have `backrefs.ttl` and `annotations.ttl`.

#### Scenario: Create new page object
- **WHEN** user runs `cerbo create "My Page"`
- **THEN** a new UUID is generated (v4)
- **THEN** directory `.cerbo/objects/<uuid>/` is created
- **THEN** `page.md` is created with `# My Page` as content
- **THEN** `meta.ttl` is created with `type: :Page` (or `:Product` - TBD)

#### Scenario: Object directory structure
- **WHEN** listing `.cerbo/objects/<uuid>/` contents
- **THEN** `meta.ttl` MUST exist
- **THEN** `page.md` exists for Page/Source/Ontology types
- **THEN** binary file exists for Attachment type
- **THEN** `backrefs.ttl` exists if object has outgoing links
- **THEN** `annotations.ttl` exists if object has HackMD annotations

### Requirement: Link Format
The system SHALL use `cerbo://<uuid>` format for all internal links in `page.md`. The system SHALL NOT include `/page.md` or `/image.png` suffix. Link type (page vs attachment) is determined by the target object's `meta.ttl` `type:` field.

#### Scenario: Page link in markdown
- **WHEN** user creates link to another page
- **THEN** the markdown link SHALL be `[Page Title](cerbo://<uuid>)`
- **THEN** the link SHALL NOT include `/page.md` suffix

#### Scenario: Attachment link in markdown
- **WHEN** user creates link to an attachment
- **THEN** the markdown link SHALL be `![Alt Text](cerbo://<uuid>)`
- **THEN** the link SHALL NOT include the filename

### Requirement: Meta TTL Structure
The system SHALL store object metadata in `meta.ttl` using Turtle RDF syntax. The file MUST contain: `type:` (`:Page`, `:Source`, `:Attachment`, `:Ontology`), `:title`, `schema:dateCreated`, `schema:dateModified`. Source types MUST include `:original-url`. Attachments MUST include `:mime-type`.

#### Scenario: Page meta.ttl content
- **WHEN** reading `meta.ttl` for a product page
- **THEN** it SHALL contain `type: :Page` (or `:Product`)
- **THEN** it SHALL contain `:title` with the page title
- **THEN** it SHALL contain `schema:dateCreated` and `schema:dateModified` as xsd:dateTime

#### Scenario: Source meta.ttl content
- **WHEN** reading `meta.ttl` for an imported source
- **THEN** it SHALL contain `type: :Source`
- **THEN** it SHALL contain `:original-url` with the source URL
- **THEN** the object SHALL be read-only

#### Scenario: Attachment meta.ttl content
- **WHEN** reading `meta.ttl` for an attachment
- **THEN** it SHALL contain `type: :Attachment`
- **THEN** it SHALL contain `:mime-type` (e.g., "image/png")

### Requirement: Relations TTL Structure
The system SHALL store outgoing links from `page.md` in `backrefs.ttl` using Turtle RDF syntax. Each `cerbo://<uuid>` link in `page.md` SHALL produce a `:linksTo` (for pages/ontologies) or `:usesAttachment` (for attachments) triple. The system SHALL cache backlinks (`:hasBacklink`) from other objects.

#### Scenario: Extract links to backrefs.ttl
- **WHEN** `page.md` contains `[Page](cerbo://<uuid-1>)` and `![img](cerbo://<uuid-2>)`
- **THEN** `backrefs.ttl` SHALL contain `:linksTo <cerbo://<uuid-1>>`
- **THEN** `backrefs.ttl` SHALL contain `:usesAttachment <cerbo://<uuid-2>>`

#### Scenario: Cache backlinks
- **WHEN** another page links to this object
- **THEN** this object's `backrefs.ttl` SHALL contain `:hasBacklink <cerbo://<other-uuid>>`

### Requirement: Annotations TTL Structure
The system SHALL extract HackMD-style `[Text]{prefix:Type}` annotations from `page.md` and store them in `annotations.ttl` using Turtle RDF syntax. Each annotation SHALL be a blank node with `:concept`, `:type`, and `:position` (line:column).

#### Scenario: Extract HackMD annotations
- **WHEN** `page.md` contains `[Berlin]{schema:Place}`
- **THEN** `annotations.ttl` SHALL contain a blank node with `:concept "Berlin"`, `:type schema:Place`

#### Scenario: Position tracking
- **WHEN** annotation starts at line 5, column 10
- **THEN** `annotations.ttl` SHALL contain `:position "5:10"`
