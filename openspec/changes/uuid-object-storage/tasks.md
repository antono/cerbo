## 1. Core Storage Layer (Rust core library)

- [x] 1.1 Add `uuid` crate dependency to `core/Cargo.toml` (already present)
- [x] 1.2 Create `core/src/object.rs` module with `ObjectType` enum (Page, Source, Attachment, Ontology)
- [x] 1.3 Implement `object_create(uuid: Option<String>, obj_type: ObjectType, title: String)` → creates `.cerbo/objects/<uuid>/` directory
- [x] 1.4 Implement `object_delete(uuid: &str)` → removes `.cerbo/objects/<uuid>/` directory (enforce read-only for Source)
- [x] 1.5 Implement `object_read(uuid: &str)` → reads `page.md` content
- [x] 1.6 Implement `object_write(uuid: &str, content: &str)` → writes `page.md` (enforce read-only for Source)
- [x] 1.7 Add Turtle RDF dependency: `rio_turtle` crate for parsing/writing `.ttl` files
 - [x] 1.8 Implement `meta_ttl_read(uuid: &str)` → parses `meta.ttl` returning `ObjectType`, title, dates, mime-type, original-url
 - [x] 1.9 Implement `meta_ttl_write(uuid: &str, type: ObjectType, title: &str, ...)` → writes `meta.ttl` in Turtle format
 - [x] 1.10 Implement `backrefs_ttl_read/write(uuid: &str)` → reads/writes `backrefs.ttl` (backlinks only)
 - [x] 1.11 Implement `annotations_ttl_write(uuid: &str, annotations: Vec<Annotation>)` → writes `annotations.ttl`

## 2. Index Management

- [x] 2.1 Create `core/src/index.rs` module with `IndexJson` struct (title_to_uuid, uuid_to_path)
- [x] 2.2 Implement `index_load()` → reads `.cerbo/index.json`
- [x] 2.3 Implement `index_save(index: &IndexJson)` → writes `.cerbo/index.json`
- [x] 2.4 Implement `index_add(title: &str, uuid: &str)` → updates both maps
- [x] 2.5 Implement `index_remove(uuid: &str)` → removes from both maps
- [x] 2.6 Implement `index_resolve_title(title: &str)` → returns Option<uuid>
- [x] 2.7 Implement `index_resolve_uuid(uuid: &str)` → returns Option<relative_path>

## 3. CLI Commands - Vault Init

- [x] 3.1 Add `cerbo init` command to CLI (parse args in `cli/src/`)
- [x] 3.2 Implement `vault_init()` → creates `.cerbo/` directory
- [x] 3.3 Implement `vault_init()` → creates `.cerbo/objects/` directory
- [x] 3.4 Implement `vault_init()` → creates empty `.cerbo/index.json`
- [x] 3.5 Implement `vault_init()` → creates `.cerbo/ontology-map.json` with empty prefixes
- [ ] 3.6 Implement bundling Schema.org ontology object (fetch content, create object with `type: Ontology`)
- [ ] 3.7 Implement bundling FOAF ontology object (fetch content, create object with `type: Ontology`)
- [ ] 3.8 Update `ontology-map.json` with "schema" → Schema.org UUID, "foaf" → FOAF UUID
- [x] 3.9 Make `cerbo init` idempotent (check for existing `.cerbo/`)

## 4. CLI Commands - Page Management

- [x] 4.1 Modify `cerbo page create "Title"` → generates UUID, creates object (NOT slug-based)
- [x] 4.2 Implement `cerbo import <url>` → creates `type: Source` object (read-only), stores original-url in `meta.ttl`
- [x] 4.3 Modify `cerbo page read <uuid>` → reads from `.cerbo/objects/<uuid>/page.md`
- [x] 4.4 Modify `cerbo page write <uuid> "content"` → writes to object's `page.md`, enforce read-only for Source type
- [x] 4.5 Modify `cerbo page delete <uuid>` → removes object directory, enforce read-only for Source type
- [x] 4.6 Modify `cerbo page list` → scans `.cerbo/objects/` for valid page objects, returns UUID + title from `meta.ttl`
- [x] 4.7 Add `cerbo resolve <uuid>` command → returns local filesystem path to object's file

## 5. CLI Commands - Ontology Management

- [x] 5.1 Add `cerbo import-ontology <url>` command
- [x] 5.2 Implement `import_ontology(url: &str)` → creates `type: Ontology` object, fetches content to `page.md`
- [x] 5.3 Update `ontology-map.json` when new ontology is imported (add prefix→uuid mapping)
- [x] 5.4 Auto-derive prefix from ontology title (e.g., "Schema.org" → "schema") or use user-provided prefix

## 6. Link Extraction and backrefs.ttl (Backlinks Only)

- [x] 6.1 Implement regex/parser to extract `cerbo://<uuid>` links from `page.md` content (outgoing links, no tracking file needed)
- [x] 6.2 On `page write`, update TARGET objects' `backrefs.ttl` with `:hasBacklink` (cached backlinks)
- [x] 6.3 Source page does NOT store outgoing links in any `.ttl` file (just in `page.md`)
- [x] 6.4 `backrefs.ttl` contains ONLY `:hasBacklink` predicates (incoming links)
- [x] 6.5 Implement `cerbo backlinks <uuid>` → reads `:hasBacklink` from object's `backrefs.ttl`

## 7. HackMD Annotation Extraction and annotations.ttl

- [x] 7.1 Implement regex/parser to extract `[Text]{prefix:Type}` syntax from `page.md`
- [x] 7.2 Resolve `prefix:` to full URI using `ontology-map.json` (e.g., `schema:` → `cerbo://objects/<uuid-schema>>`)
- [x] 7.3 On `page write`, extract annotations and write to `annotations.ttl` as Turtle RDF blank nodes
- [x] 7.4 Track annotation position (line:column) in `page.md` and store in `:position`
- [x] 7.5 Define `cerbo://ontology/` prefixes for `:concept`, `:type`, `:position`

## 8. Attachment Management (Modified)

- [x] 8.1 Modify attachment commands → create separate `type: Attachment` object (NOT `<slug>/assets/`)
- [x] 8.2 Implement `attachment_add(page_uuid: &str, file_path: &Path)` → creates attachment object, copies file, returns UUID
- [x] 8.3 Implement `attachment_delete(attachment_uuid: &str)` → removes attachment object directory
- [x] 8.4 Modify `attachment_list(page_uuid: &str)` → reads page's `backrefs.ttl` for `:usesAttachment`
- [x] 8.5 Set `:mime-type` in attachment's `meta.ttl` (detect from file or user-provided)

## 9. Remove Slug-Based Code

- [x] 9.1 Remove or refactor `core/src/slug.rs` (slug derivation no longer needed)
- [x] 9.2 Update all references from `slug` to `uuid` in `core/src/page.rs`
- [x] 9.3 Update all references from `<slug>/page.md` to `.cerbo/objects/<uuid>/page.md`
- [x] 9.4 Remove wikilink `[[Title]]` parsing from `core/src/index.rs` (replace with `cerbo://` link parsing)
- [x] 9.5 Update `core/src/vault.rs` → remove slug-based vault-add, keep UUID-based operations

## 10. Read-Only Enforcement

- [x] 10.1 Check `meta.ttl` `type: :Source` before allowing `page write` → return error if Source
- [x] 10.2 Check `meta.ttl` `type: :Source` before allowing `page delete` → return error if Source
- [x] 10.3 Return descriptive error messages: "Cannot write to source type (read-only)"

## 11. Testing

- [x] 11.1 Write unit tests for `object_create` / `object_delete`
- [x] 11.2 Write unit tests for `index_resolve_title` / `index_resolve_uuid`
- [x] 11.3 Write unit tests for link extraction (`cerbo://<uuid>`)
- [x] 11.4 Write unit tests for HackMD annotation extraction (`[Text]{prefix:Type}`)
- [ ] 11.5 Write integration test: `cerbo init` → verify `.cerbo/` structure
- [ ] 11.6 Write integration test: `cerbo create` → verify UUID object created with `meta.ttl`
- [ ] 11.7 Write integration test: `cerbo import` → verify `type: Source`, read-only enforcement
- [ ] 11.8 Test migration path: verify old vaults are NOT compatible (breaking change by design)

## 12. Documentation

- [ ] 12.1 Update CLI help text to reflect new commands (`cerbo init`, `cerbo import`, `cerbo resolve`)
- [ ] 12.2 Document storage layout in `README.md` or `docs/storage.md`
- [ ] 12.3 Document HackMD annotation syntax for users
- [ ] 12.4 Document `cerbo://<uuid>` link format
