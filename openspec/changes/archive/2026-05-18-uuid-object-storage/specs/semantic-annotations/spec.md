# Semantic Annotations

## Purpose
Define how HackMD-style semantic markdown annotations are extracted and stored in `annotations.ttl`.

## ADDED Requirements

### Requirement: HackMD Annotation Syntax
The system SHALL support HackMD-style semantic annotations in `page.md` using the syntax `[Text]{prefix:Type}`. This syntax SHALL NOT create a markdown link. The system SHALL extract these annotations and store them in `annotations.ttl`.

#### Scenario: Basic annotation
- **WHEN** `page.md` contains `[Berlin]{schema:Place}`
- **THEN** the text "Berlin" SHALL be displayed (not a link)
- **THEN** `annotations.ttl` SHALL contain a triple with `:concept "Berlin"` and `:type schema:Place`

#### Scenario: Annotation with custom prefix
- **WHEN** `page.md` contains `[Alice]{foaf:Person}`
- **THEN** `annotations.ttl` SHALL contain `:type foaf:Person` (resolved from `ontology-map.json`)

### Requirement: Annotation Extraction
The system SHALL extract HackMD annotations from `page.md` when the page is saved (via CLI or editor). Each annotation SHALL be stored as a blank node in `annotations.ttl` with `:concept` (text), `:type` (resolved predicate), and `:position` (line:column in `page.md`).

#### Scenario: Extract and store annotations
- **WHEN** `page.md` is saved with content `I visited [Berlin]{schema:Place}.`
- **THEN** `annotations.ttl` SHALL be created/updated
- **THEN** it SHALL contain:
  ```turtle
  <cerbo://objects/<uuid>> :hasAnnotation [
      :concept "Berlin" ;
      :type schema:Place ;
      :position "1:10"
  ] .
  ```

#### Scenario: Multiple annotations
- **WHEN** `page.md` contains multiple `[Text]{prefix:Type}` annotations
- **THEN** `annotations.ttl` SHALL contain one `:hasAnnotation` blank node per annotation

### Requirement: Annotation Rendering
The desktop editor SHALL render HackMD annotations as styled text (not links). The rendering SHALL visually distinguish annotations from regular text and links.

#### Scenario: Render annotation in editor preview
- **WHEN** `page.md` contains `[Berlin]{schema:Place}`
- **THEN** the preview SHALL display "Berlin" with visual annotation styling
- **THEN** the preview SHALL NOT create a clickable link

**Note**: Editor behavior details are out of scope; this requirement defines the expected output only.
