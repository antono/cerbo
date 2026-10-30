# Tasks

## 1. Vocabulary core

- [x] 1.1 In `core/src/rdf.rs` add `NS_RDF`, change `NS_SCHEMA` to `https://schema.org/`, add a `rdf_type()` helper and the legacy alias table from design D2 (predicates and the four legacy classes); declare `rdf:` in the serializer; verify unit tests that the serializer output parses and uses `rdf:type`/`https://schema.org/`
- [x] 1.2 Make `rdf::parse` rewrite legacy IRIs (`cerbo:type`, `cerbo:title`, `cerbo:original-url`, `cerbo:mime-type`, `cerbo://ontology/schema/*`, `cerbo:Product`/`Source`/`Attachment`/`Ontology`, `cerbo:annotation`/`cerbo:concept`); verify a test that a legacy `meta.ttl` from `test-vault` reads with the same type, title and dates
- [x] 1.3 Switch `ObjectMeta` write/read in `core/src/object.rs` to `rdf:type` with the schema.org classes from design D1, `schema:name`, `schema:url` (IRI; literal accepted on read), `schema:encodingFormat`; reject two or more of the four classes with an error naming the UUID; verify the `uuid-object-storage` scenarios including "Ambiguous object class" and "Extra unrelated class"

## 2. Prefix map and annotation types

- [x] 2.1 Implement the v2 `ontology-map.json` reader (v1 string values accepted) and the built-in prefix table, resolving from the vault's `.cerbo/` via `VaultContext`; verify unit tests for map entry, built-in, override, v1 file, and two vaults with different `ex`
- [x] 2.2 Rewrite annotation serialisation: `<page> schema:mentions _:m` with `schema:name`, `rdf:type <full IRI>`, `cerbo:unresolvedType` for unknown prefixes plus a warning, `cerbo:position "line:column"`, and always rewrite `annotations.ttl` (empty when no annotations); verify the `semantic-annotations` scenarios including "Last annotation removed"
- [x] 2.3 Add `--prefix`/`--namespace` to `cerbo import-ontology` (both or neither), writing the v2 map; make `cerbo init` write `{"version": 2, "prefixes": {}}`; verify CLI tests for the three import scenarios

## 3. Migration command

- [x] 3.1 Implement `migrate_vocabulary(vault_ctx, dry_run) -> MigrationReport` in core: tolerant read + reserialise `meta.ttl`, regenerate `annotations.ttl` from `page.md`, byte-compare, atomic write, skip trash, collect per-UUID errors; verify tests for legacy vault, idempotence (no mtime change) and one unreadable object among 137
- [x] 3.2 Add `cerbo migrate vocabulary [--vault] [--dry-run] [--json]` with the summary and exit codes; verify CLI tests for every `vocabulary-migration` scenario
- [x] 3.3 Run the migration on `test-vault` and commit the result together with the implementation; verify `git diff` shows only vocabulary changes

## 4. Docs and checks

- [x] 4.1 Document `cerbo migrate vocabulary`, the new `import-ontology` flags, the built-in prefixes and the "upgrade all devices first" warning in `cli/man/cerbo.md`; update `docs/storage.md` and `README.md` examples; verify `nix build .#cerbo` builds the man page
- [x] 4.2 Run `cargo test -p cerbo-core`, `cargo test -p cerbo` (inside `nix develop`), `nix build .#cerbo .#cerbo-desktop` and `openspec validate normalize-rdf-vocabulary --strict`; verify all pass
