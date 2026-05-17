# Ontology Management

## Purpose
Define how ontologies (Schema.org, FOAF, custom) are managed as first-class objects in Cerbo.

## Requirements

### Requirement: Ontology as Object
The system SHALL treat ontologies as first-class objects stored under `.cerbo/objects/<uuid>/` with `type: :Ontology` in `meta.ttl`. Ontology objects SHALL have `page.md` (containing documentation) and MAY have `:original-url` pointing to the ontology source.

#### Scenario: Bundled ontology on init
- **WHEN** user runs `cerbo init`
- **THEN** Schema.org ontology object SHALL be created
- **THEN** FOAF ontology object SHALL be created
- **THEN** both SHALL have `type: :Ontology` in `meta.ttl`

#### Scenario: Import custom ontology
- **WHEN** user runs `cerbo import-ontology <url>`
- **THEN** a new object with `type: :Ontology` SHALL be created
- **THEN** `meta.ttl` SHALL contain `:original-url` with the provided URL
- **THEN** `page.md` SHALL contain the fetched ontology documentation

### Requirement: Prefix Mapping
The system SHALL maintain `.cerbo/ontology-map.json` mapping prefixes (e.g., "schema", "foaf") to ontology object UUIDs. The system SHALL use this mapping to resolve `prefix:` in HackMD annotations like `[Text]{schema:Place}`.

#### Scenario: Resolve prefix in annotation
- **WHEN** `ontology-map.json` contains `{ "schema": "<uuid-schema-org>" }`
- **THEN** `[Text]{schema:Place}` SHALL resolve `schema:` to `<cerbo://objects/<uuid-schema-org>>`
- **THEN** the annotation type SHALL be `<cerbo://objects/<uuid-schema-org>>:Place`

#### Scenario: Auto-register imported ontology
- **WHEN** user runs `cerbo import-ontology https://schema.org/docs/schemas.html`
- **THEN** `ontology-map.json` SHALL be updated with prefix "schema" (or derived prefix)
- **THEN** subsequent `[Text]{schema:Type}` annotations SHALL use the new ontology

### Requirement: Ontology Object Storage
Ontology objects SHALL follow the same storage layout as pages. The `meta.ttl` MUST contain `type: :Ontology`. The `page.md` contains human-readable documentation about the ontology.

#### Scenario: Ontology object structure
- **WHEN** reading an ontology object at `.cerbo/objects/<uuid-ontology>/`
- **THEN** `meta.ttl` SHALL contain `type: :Ontology`
- **THEN** `page.md` SHALL exist with documentation content
- **THEN** `relations.ttl` MAY exist (if ontology links to other resources)
- **THEN** `annotations.ttl` MAY exist (if documentation has HackMD annotations)
