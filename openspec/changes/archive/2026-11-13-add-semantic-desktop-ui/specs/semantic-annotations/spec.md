# Spec Delta

## MODIFIED Requirements

### Requirement: Annotation Rendering
The desktop editor SHALL render HackMD annotations as styled text (not links). The rendering SHALL visually distinguish annotations from regular text and links. Each annotation SHALL show its type on hover, as written (`schema:Place`) and as the resolved full IRI. An annotation whose prefix cannot be resolved for the vault SHALL be styled as a warning and its tooltip SHALL say that the prefix is unknown.

#### Scenario: Render annotation in editor preview
- **WHEN** `page.md` contains `[Berlin]{schema:Place}`
- **THEN** the preview SHALL display "Berlin" with visual annotation styling
- **THEN** the preview SHALL NOT create a clickable link

#### Scenario: Type tooltip
- **WHEN** the user hovers "Berlin" rendered from `[Berlin]{schema:Place}`
- **THEN** a tooltip SHALL show `schema:Place` and `https://schema.org/Place`

#### Scenario: Unknown prefix
- **WHEN** `page.md` contains `[Thing]{zz:Widget}` and `zz` is unknown in the vault
- **THEN** "Thing" SHALL be rendered with warning styling
- **THEN** its tooltip SHALL say that the prefix `zz` is unknown

#### Scenario: Annotation syntax is not rendered literally
- **WHEN** the preview renders `[Berlin]{schema:Place}`
- **THEN** the braces and the type SHALL NOT appear as text in the preview

**Note**: Editor behavior details are out of scope; this requirement defines the expected output only.
