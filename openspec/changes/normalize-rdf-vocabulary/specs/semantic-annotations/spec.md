# Spec Delta

## MODIFIED Requirements

### Requirement: HackMD Annotation Syntax
The system SHALL support HackMD-style semantic annotations in `page.md` using the syntax `[Text]{prefix:Type}`. This syntax SHALL NOT create a markdown link. The system SHALL extract these annotations and store each one in `annotations.ttl` as an entity the page mentions (`schema:mentions`), typed with the full IRI formed by the prefix's namespace IRI followed by `Type`.

#### Scenario: Basic annotation
- **WHEN** `page.md` contains `[Berlin]{schema:Place}`
- **THEN** the text "Berlin" SHALL be displayed (not a link)
- **THEN** `annotations.ttl` SHALL state that the page `schema:mentions` a node with `schema:name "Berlin"` and `rdf:type <https://schema.org/Place>`

#### Scenario: Annotation with custom prefix
- **WHEN** the vault's prefix map maps `ex` to `https://example.org/vocab#`
- **AND** `page.md` contains `[Alice]{ex:Person}`
- **THEN** `annotations.ttl` SHALL contain `rdf:type <https://example.org/vocab#Person>`

#### Scenario: Well-known prefix without a map entry
- **WHEN** the vault's prefix map has no `foaf` entry and `page.md` contains `[Alice]{foaf:Person}`
- **THEN** the annotation SHALL be typed `<http://xmlns.com/foaf/0.1/Person>`

### Requirement: Annotation Extraction
The system SHALL extract HackMD annotations from `page.md` when the page is saved (via CLI or editor). Each annotation SHALL be stored as a blank node in `annotations.ttl` that the page `schema:mentions`, with `schema:name` (text), `rdf:type` (resolved type IRI), and `cerbo:position` (`line:column` in `page.md`). A page saved without annotations SHALL leave no annotation triples behind.

#### Scenario: Extract and store annotations
- **WHEN** `page.md` is saved with content `I visited [Berlin]{schema:Place}.`
- **THEN** `annotations.ttl` SHALL be created/updated
- **THEN** it SHALL be equivalent to:
  ```turtle
  <cerbo://objects/<uuid>> schema:mentions [
      schema:name "Berlin" ;
      rdf:type <https://schema.org/Place> ;
      cerbo:position "1:10"
  ] .
  ```

#### Scenario: Mentions are queryable with schema.org alone
- **WHEN** a page contains `[Berlin]{schema:Place}`
- **THEN** `SELECT ?page ?name WHERE { ?page <https://schema.org/mentions> [ a <https://schema.org/Place> ; <https://schema.org/name> ?name ] }` over its files SHALL return the page and "Berlin"

#### Scenario: Multiple annotations
- **WHEN** `page.md` contains multiple `[Text]{prefix:Type}` annotations
- **THEN** `annotations.ttl` SHALL contain one `schema:mentions` blank node per annotation

#### Scenario: Last annotation removed
- **WHEN** a page that had annotations is saved without any
- **THEN** its `annotations.ttl` SHALL contain no `schema:mentions` triple

## ADDED Requirements

### Requirement: Unresolvable prefixes are kept and reported

When an annotation's prefix is neither in the vault's prefix map nor well known, the system SHALL keep the mentioned node with `schema:name`, `cerbo:position` and `cerbo:unresolvedType "prefix:Type"` instead of `rdf:type`. It SHALL NOT invent an IRI for it, and SHALL report a warning naming the prefix. The save SHALL succeed.

#### Scenario: Unknown prefix
- **WHEN** `page.md` contains `[Thing]{zz:Widget}` and `zz` is not known
- **THEN** `annotations.ttl` SHALL contain `cerbo:unresolvedType "zz:Widget"` for that annotation
- **THEN** it SHALL contain no `rdf:type` for that annotation
- **THEN** a warning SHALL name the prefix `zz`

#### Scenario: Prefix becomes known later
- **WHEN** the prefix `zz` is added to the vault's prefix map and the page is reindexed
- **THEN** the annotation SHALL be typed with the full IRI and SHALL lose `cerbo:unresolvedType`
