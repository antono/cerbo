# Spec Delta

## MODIFIED Requirements

### Requirement: Prefix Mapping
The system SHALL maintain a per-vault `.cerbo/ontology-map.json` mapping each prefix (e.g., "schema", "ex") to a namespace IRI and, optionally, the UUID of the ontology object that documents it. Annotation resolution SHALL read only the map of the vault the page belongs to. The system SHALL use this mapping to resolve `prefix:` in HackMD annotations like `[Text]{schema:Place}` to the namespace IRI followed by the local name.

#### Scenario: Resolve prefix in annotation
- **WHEN** `ontology-map.json` maps `schema` to namespace `https://schema.org/` and ontology `<uuid-schema-org>`
- **THEN** `[Text]{schema:Place}` SHALL resolve to `<https://schema.org/Place>`
- **THEN** the ontology object `<uuid-schema-org>` SHALL NOT appear in the annotation type IRI

#### Scenario: Map belongs to the vault
- **WHEN** two vaults map the prefix `ex` to different namespaces
- **THEN** annotations in each vault SHALL resolve `ex:` using that vault's own map

#### Scenario: Auto-register imported ontology
- **WHEN** user runs `cerbo import-ontology <url> --prefix ex --namespace https://example.org/vocab#`
- **THEN** `ontology-map.json` SHALL map `ex` to `https://example.org/vocab#` and the new ontology object's UUID
- **THEN** subsequent `[Text]{ex:Type}` annotations SHALL resolve to `<https://example.org/vocab#Type>`

#### Scenario: Import without a namespace
- **WHEN** user runs `cerbo import-ontology <url>` without `--prefix` and `--namespace`
- **THEN** the ontology object SHALL be created
- **THEN** `ontology-map.json` SHALL NOT change
- **THEN** a warning SHALL say that annotations cannot use the ontology until it is registered with `--prefix` and `--namespace`

#### Scenario: Only one of the two flags
- **WHEN** user runs `cerbo import-ontology <url> --prefix ex` without `--namespace`
- **THEN** the command SHALL exit non-zero and create no object

#### Scenario: Legacy map format
- **WHEN** `ontology-map.json` maps a prefix only to an ontology UUID, as written by earlier versions
- **THEN** a well-known prefix SHALL resolve to its standard namespace IRI
- **THEN** any other prefix SHALL be treated as unresolvable and reported

## ADDED Requirements

### Requirement: Well-known prefixes are built in

The prefixes `schema` (`https://schema.org/`), `foaf` (`http://xmlns.com/foaf/0.1/`), `dcterms` (`http://purl.org/dc/terms/`), `skos` (`http://www.w3.org/2004/02/skos/core#`), `rdf`, `rdfs`, `owl` and `xsd` (their W3C namespaces) SHALL resolve without any map entry. A vault map entry for the same prefix SHALL take precedence.

#### Scenario: Fresh vault resolves schema
- **WHEN** a vault's `ontology-map.json` is empty
- **THEN** `[Berlin]{schema:Place}` SHALL resolve to `<https://schema.org/Place>`

#### Scenario: Vault overrides a built-in prefix
- **WHEN** the vault maps `schema` to `https://example.org/schema/`
- **THEN** `[Berlin]{schema:Place}` SHALL resolve to `<https://example.org/schema/Place>`
