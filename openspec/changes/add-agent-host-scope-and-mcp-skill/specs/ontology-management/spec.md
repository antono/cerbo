# Spec Delta

## ADDED Requirements

### Requirement: Vault-owned ontologies

A vault SHALL be able to own an ontology that it creates itself, in the vault's own namespace, with its own prefix registered in `.cerbo/ontology-map.json`. Types SHALL be addable to such an ontology after it was created, each recorded as a class declaration in the ontology object's `meta.ttl` with a label and optional description and parent. Ontologies that are bundled or imported SHALL NOT accept added types.

#### Scenario: Create and extend
- **WHEN** a vault-owned ontology with prefix `ex` is created and the type `Recipe` is added
- **THEN** `meta.ttl` of that ontology SHALL declare `Recipe` and `[Pancakes]{ex:Recipe}` SHALL resolve to the vault's namespace followed by `Recipe`

#### Scenario: Taken prefix
- **WHEN** an ontology is created with a prefix that the map already uses
- **THEN** the creation SHALL fail and nothing SHALL change

#### Scenario: Imported ontology
- **WHEN** a type is added to an ontology imported from a URL
- **THEN** the addition SHALL be refused
