# Spec Delta

## Purpose

Bring an existing Obsidian or Tolaria vault into a Cerbo vault in one step,
keeping notes, folders, links, attachments and frontmatter meaning, without
modifying the source folder.

## ADDED Requirements

### Requirement: Source format detection
The system SHALL detect the source format before importing. A folder that
contains `.obsidian/` is Obsidian. A folder without `.obsidian/` that has a
`views/` folder of `.yml` files, or Markdown notes with a `type:` frontmatter
field, is Tolaria. `--from obsidian|tolaria` SHALL override detection.

#### Scenario: Obsidian detected
- **WHEN** user runs `cerbo import-vault ~/Notes` and `~/Notes/.obsidian/` exists
- **THEN** the import runs with Obsidian rules

#### Scenario: Tolaria detected
- **WHEN** the source has no `.obsidian/` and has `views/devices.yml`
- **THEN** the import runs with Tolaria rules

#### Scenario: Undetectable folder
- **WHEN** the source matches neither format and `--from` is not given
- **THEN** the command exits non-zero with "cannot detect vault format, pass --from obsidian|tolaria"
- **THEN** nothing is written

#### Scenario: Source is a Cerbo vault
- **WHEN** the source folder contains `.cerbo/`
- **THEN** the command exits non-zero and writes nothing

### Requirement: Source folder is read-only
The import SHALL NOT create, modify, move or delete any file under the
source folder.

#### Scenario: Source unchanged after import
- **WHEN** an import completes
- **THEN** every file under the source folder has the same content and modification time as before

### Requirement: Interactive target selection
Without `--vault` or `--new`, on a terminal, the command SHALL ask where to
import: a numbered list of registered vaults (name and path) plus "Create a
new vault", which then asks for a path and a name.

#### Scenario: Choose a registered vault
- **WHEN** user runs `cerbo import-vault ~/Notes` on a terminal and picks vault 2
- **THEN** notes are imported into the second listed vault

#### Scenario: Create a new vault interactively
- **WHEN** user picks "Create a new vault" and enters `~/cerbo/notes` and name `Notes`
- **THEN** the folder is initialised as with `cerbo init`, registered as `Notes`, and the import goes there

#### Scenario: No terminal and no target flag
- **WHEN** stdin is not a terminal and neither `--vault` nor `--new` is given
- **THEN** the command exits non-zero with "no target vault: pass --vault <ID> or --new <PATH>"
- **THEN** nothing is written

### Requirement: Non-interactive target selection
`--vault <ID>` SHALL import into a registered vault. `--new <PATH>` SHALL
initialise and register a vault at `PATH` (named `--name`, default the source
folder name) and import into it. The two flags SHALL be mutually exclusive.

#### Scenario: Import into a registered vault
- **WHEN** user runs `cerbo import-vault ~/Notes --vault 3f2a`
- **THEN** no prompt is shown and notes go into vault `3f2a`

#### Scenario: New vault in a non-empty folder
- **WHEN** `--new <PATH>` names a folder that contains files other than `.git`
- **THEN** the command exits non-zero and writes nothing

#### Scenario: Unknown vault id
- **WHEN** `--vault` names an id that is not registered
- **THEN** the command exits non-zero with "Vault not found" and writes nothing

### Requirement: Notes become pages
Every Markdown file in the source, except skipped files, SHALL become one
`:Product` page. Its folder (relative to the source root) SHALL become
`cerbo:virtualPath` and its file stem, slugified, SHALL become `cerbo:slug`.

#### Scenario: Nested note
- **WHEN** the source has `journals/2025-04-27.md`
- **THEN** a page is created with `cerbo:virtualPath "journals"` and `cerbo:slug "2025-04-27"`

#### Scenario: Non-ASCII file name
- **WHEN** the source has `филаменты.md`
- **THEN** the page's slug is the ASCII transliteration of `филаменты`

#### Scenario: Slug collision in a folder
- **WHEN** two notes in one folder slugify to the same slug
- **THEN** the second gets the suffix `-2` (then `-3`, …) and the report lists the rename

### Requirement: Page title
For Tolaria the title SHALL be the first H1 of the body, falling back to the
file stem. For Obsidian the title SHALL be the file stem. When the body does
not start with that H1, `# <title>` SHALL be prepended to `page.md`.

#### Scenario: Tolaria title from H1
- **WHEN** `devices.md` starts with `# Devices` after its frontmatter
- **THEN** the page title is `Devices` and the H1 appears once in `page.md`

#### Scenario: Obsidian title from file name
- **WHEN** Obsidian note `Rust Ownership.md` has no H1
- **THEN** the page title is `Rust Ownership` and `page.md` starts with `# Rust Ownership`

### Requirement: Page dates
`schema:dateCreated` SHALL be the file's birth time when the filesystem
reports one, otherwise its modification time. `schema:dateModified` SHALL be
the modification time.

#### Scenario: Dates kept
- **WHEN** a note was last modified on 2025-12-07
- **THEN** its page has `schema:dateModified` on 2025-12-07, not the import date

### Requirement: Skipped files
The import SHALL skip `.obsidian/`, `.git/`, `.trash/`, Tolaria `views/`,
hidden files and folders, and Obsidian `.canvas` files, and SHALL list the
skipped non-hidden files in the report.

#### Scenario: Canvas skipped
- **WHEN** the source has `plan.canvas`
- **THEN** no object is created for it and the report lists it as skipped

### Requirement: Wikilink rewriting
`[[Target]]`, `[[Target|Text]]` and `[[Target#Heading]]` SHALL be rewritten to
`[Text](cerbo://<uuid>)`, where the link text is the alias, else the target's
page title. A target SHALL match a note by relative path, then file stem,
then title, then Obsidian alias, case-insensitively.

#### Scenario: Link by file stem
- **WHEN** a note contains `[[orange-pi-zero-2w-2g-octoprint]]`
- **THEN** it becomes `[Orange PI Zero 2W 2G - Octoprint](cerbo://<uuid>)`

#### Scenario: Link by title with alias
- **WHEN** a note contains `[[Desktop PC|my PC]]` and a note titled `Desktop PC` exists
- **THEN** it becomes `[my PC](cerbo://<uuid of Desktop PC>)`

#### Scenario: Ambiguous target
- **WHEN** two notes in different folders have the stem `index` and a link says `[[index]]`
- **THEN** the link resolves to the one closest to the linking note's folder, and the report lists it as ambiguous

#### Scenario: Unresolved link
- **WHEN** a note contains `[[Missing Note]]` and no note matches
- **THEN** the text `[[Missing Note]]` is kept as is and the report lists it as unresolved

#### Scenario: Note embed
- **WHEN** a note contains `![[Other Note]]`
- **THEN** it becomes an ordinary link `[Other Note](cerbo://<uuid>)`

#### Scenario: Wikilinks inside code
- **WHEN** `[[X]]` appears inside a fenced code block or inline code
- **THEN** it is left unchanged

### Requirement: Markdown links to notes
Relative Markdown links to another note in the source, such as
`[text](other.md)` or `[text](other%20note.md)`, SHALL be rewritten to
`[text](cerbo://<uuid>)`. External URLs SHALL be left unchanged.

#### Scenario: Relative note link
- **WHEN** a note contains `[see](../devices.md)` and that note is imported
- **THEN** it becomes `[see](cerbo://<uuid of devices>)`

### Requirement: Attachments
A non-Markdown file referenced from a note by `![[file]]`, `![](path)` or
`[](path)` SHALL become one Attachment object, linked from every referencing
page and recorded in its relations. Each file SHALL be imported once however
many notes use it. Unreferenced files SHALL be listed in the report, not imported.

#### Scenario: Obsidian image embed
- **WHEN** a note contains `![[photo.png]]` and `attachments/photo.png` exists
- **THEN** an Attachment object with the file is created and the embed becomes `![photo.png](cerbo://<uuid>)`

#### Scenario: Tolaria attachment link
- **WHEN** a note contains `[manual](attachments/pdf/BIGTREETECH-Octopus-1.1-color-PIN.pdf)`
- **THEN** an Attachment object holds the PDF and the link becomes `[manual](cerbo://<uuid>)`

#### Scenario: Shared attachment
- **WHEN** two notes embed the same image
- **THEN** one Attachment object is created and both pages link to it

### Requirement: Frontmatter type
A frontmatter `type` value (Tolaria) SHALL add a second `rdf:type` to the
page, a class in the import's namespace, next to `schema:NoteDigitalDocument`.
Tolaria notes with `type: Type` SHALL also declare that class with its title as
label in the import's ontology.

#### Scenario: Journal note
- **WHEN** `journals/2025-04-27.md` has `type: Journal`
- **THEN** its `meta.ttl` has `rdf:type schema:NoteDigitalDocument` and `rdf:type <ns>Journal`

### Requirement: Frontmatter relationships
A frontmatter property whose value contains `[[wikilinks]]` (string or list)
SHALL become one triple per resolved link, with the predicate `<ns><key>` and
the target `cerbo://objects/<uuid>`. Unresolved links SHALL be stored as
string literals and reported.

#### Scenario: Tolaria relationship
- **WHEN** a note has `belongs_to: "[[3d-printing]]"`
- **THEN** its `meta.ttl` has `<ns>belongs_to <cerbo://objects/<uuid of 3D Printing>>`

#### Scenario: List relationship
- **WHEN** a note has `related_to:` with two wikilinks in a YAML list
- **THEN** `meta.ttl` has two `<ns>related_to` triples

### Requirement: Frontmatter literals
Other frontmatter values SHALL become literals with predicate `<ns><key>`:
strings as `xsd:string`, booleans as `xsd:boolean`, integers as `xsd:integer`,
ISO dates as `xsd:date`/`xsd:dateTime`, list items as one triple each. Keys
starting with `_` SHALL be dropped. Nested maps SHALL be kept as a YAML string.

#### Scenario: Status and URL
- **WHEN** a note has `status: Active` and `url: https://example.com`
- **THEN** `meta.ttl` has `<ns>status "Active"` and `<ns>url "https://example.com"`

#### Scenario: Tolaria app state dropped
- **WHEN** a note has `_favorite: true` and `_favorite_index: 4`
- **THEN** neither appears in `meta.ttl`

### Requirement: Obsidian tags and aliases
Obsidian `tags` (frontmatter, with or without `#`) SHALL become
`schema:keywords` literals. `aliases` SHALL become `schema:alternateName`
literals and SHALL be used to resolve wikilinks. Inline `#tags` in the body
SHALL stay as text.

#### Scenario: Tags
- **WHEN** a note has `tags: [rust, "#lang"]`
- **THEN** `meta.ttl` has `schema:keywords "rust"` and `schema:keywords "lang"`

### Requirement: Frontmatter removed from page body
`page.md` SHALL contain the note body without its YAML frontmatter.

#### Scenario: Body only
- **WHEN** a note starts with a `---` frontmatter block
- **THEN** `page.md` starts at the first line after the closing `---`

### Requirement: Import ontology
Each import SHALL create one Ontology object titled `<Format>: <source folder
name>`, recording the source path, format and import time, whose namespace is
`cerbo://vocab/<uuid>/`. Its prefix (the source folder name, slugified,
made unique) SHALL be registered in the vault's prefix map.

#### Scenario: Prefix registered
- **WHEN** `~/Sync/Tolaria/Braindump` is imported
- **THEN** an Ontology object `Tolaria: Braindump` exists and `ontology-map.json` maps `braindump` to its namespace

#### Scenario: Prefix taken
- **WHEN** the prefix `braindump` is already in the map
- **THEN** the import uses `braindump-2`

### Requirement: One import per source and vault
Importing a source folder into a vault that already holds an import of the
same canonical source path SHALL be refused. `--force` SHALL create a second,
independent copy with its own ontology and prefix.

#### Scenario: Repeat refused
- **WHEN** user imports `~/Notes` into vault `3f2a` twice
- **THEN** the second run exits non-zero naming the earlier import's date and writes nothing

#### Scenario: Forced repeat
- **WHEN** the second run has `--force`
- **THEN** a second set of pages is created and the first is untouched

### Requirement: All-or-nothing write
The import SHALL make no object visible in the target vault until all
objects are written. A failure SHALL leave the target vault's live objects and
prefix map as they were, except a vault created by `--new`, which is kept empty.

#### Scenario: Failure mid-import
- **WHEN** the disk fills while writing attachments
- **THEN** the command exits non-zero, `cerbo page list` shows the same pages as before, and no staging data is left in `.cerbo/objects/`

### Requirement: Index after import
After a successful import the semantic index SHALL be brought up to date, so
backlinks and search include the imported pages without running `cerbo index`.

#### Scenario: Backlinks right after import
- **WHEN** an import finishes and `devices.md` linked to `Desktop PC`
- **THEN** `cerbo page backlinks <uuid of Desktop PC>` lists `Devices`

### Requirement: Dry run
`--dry-run` SHALL do detection, scanning and link resolution and print the
report without asking for or writing to any vault.

#### Scenario: Dry run writes nothing
- **WHEN** user runs `cerbo import-vault ~/Notes --dry-run`
- **THEN** the report is printed, no prompt is shown, and no vault or registry file changes

### Requirement: Import report
The command SHALL print the target vault, counts of pages, attachments and
relations created, and lists of skipped files, unreferenced files,
unresolved and ambiguous links and renamed slugs. With `--json` it SHALL print
only this report as one JSON object.

#### Scenario: Human report
- **WHEN** an import of 137 notes finishes with 2 unresolved links
- **THEN** stdout says `Imported 137 pages` and lists both unresolved links with the note they are in

#### Scenario: JSON report
- **WHEN** the command runs with `--json`
- **THEN** stdout is one JSON object with `vault`, `format`, `pages`, `attachments`, `relations`, `skipped`, `unreferenced`, `unresolved`, `ambiguous`, `renamedSlugs`
