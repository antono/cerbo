# Spec Delta

## Purpose

Maintains a disposable, per-vault, per-machine RDF graph index derived from vault files, so the vault's knowledge graph, including backlinks, can be queried as a whole while plain files remain the only source of truth.

## ADDED Requirements

### Requirement: Index lives in the user cache and is derived

The system SHALL keep each vault's semantic index in the user's cache directory (`$XDG_CACHE_HOME/cerbo/index/<vault-key>/`), never inside the vault, where `<vault-key>` is derived from the vault's canonical path. The index SHALL hold only data derived from vault files, so deleting it SHALL lose no user data.

#### Scenario: Deleting the index loses nothing
- **WHEN** the user deletes the vault's index directory and runs `cerbo index`
- **THEN** the rebuilt index SHALL answer every query exactly as before the deletion

#### Scenario: Nothing is written into the vault
- **WHEN** the index is built, updated or queried
- **THEN** no file under the vault directory SHALL be created or modified by the index

#### Scenario: Two vaults have separate indexes
- **WHEN** two vaults are indexed on the same machine
- **THEN** each SHALL have its own index directory, and a query in one SHALL NOT return objects of the other

### Requirement: Index contains every live object's graph

For every live object, the index SHALL contain the triples of its `meta.ttl` and `annotations.ttl`, and one `schema:citation` triple from the object to each object its `page.md` links to with `cerbo://<uuid>`, whether or not the target exists. Ontology objects SHALL be included. Trashed objects SHALL NOT appear in the index.

#### Scenario: Object metadata is queryable
- **WHEN** an object's `meta.ttl` states `schema:name "Index"`
- **THEN** the index SHALL contain that triple with the object's `cerbo://objects/<uuid>` IRI as subject

#### Scenario: Outgoing link is queryable
- **WHEN** page A's `page.md` contains `[B](cerbo://<uuid-b>)` and B exists
- **THEN** the index SHALL contain `<cerbo://objects/<uuid-a>> schema:citation <cerbo://objects/<uuid-b>>`

#### Scenario: Broken link is queryable
- **WHEN** page A links to a UUID that is not a live object
- **THEN** the index SHALL contain the `schema:citation` triple
- **THEN** the index SHALL contain no triple with that UUID's IRI as subject, so the link can be found as broken

#### Scenario: Annotation is queryable
- **WHEN** a page contains `[Berlin]{schema:Place}` and has been indexed
- **THEN** the index SHALL contain that page's annotation triples

#### Scenario: Trashed object is absent
- **WHEN** an object is moved to the trash
- **THEN** none of its triples SHALL remain in the index, including its outgoing links

### Requirement: Index follows page create, write and delete

When a page is created, written or deleted through Cerbo, the system SHALL replace that object's triples in the index. It SHALL NOT change any other object's triples, and SHALL NOT write any other object's files.

#### Scenario: New link appears in the index
- **WHEN** page A is saved with a new `cerbo://<uuid-b>` link
- **THEN** a query for pages linking to B SHALL return A without a manual reindex
- **THEN** no file of object B SHALL be modified

#### Scenario: Removed annotation leaves the index
- **WHEN** a page is saved without an annotation it previously had
- **THEN** the index SHALL no longer contain that annotation

#### Scenario: Link to a page created later
- **WHEN** page A links to `cerbo://<uuid-c>` before C exists, and C is created later with that UUID
- **THEN** after C is created the index SHALL contain `A schema:citation C`

### Requirement: Index catches up with changes made outside Cerbo

Before answering a query, the system SHALL detect objects whose files were added, changed or removed since they were last indexed, including changes that arrived through git, file sync or manual editing, and SHALL update their triples first.

#### Scenario: Page arrives through file sync
- **WHEN** another device adds page D linking to B, and the sync tool copies D's files into the vault
- **AND** the user then asks for B's backlinks
- **THEN** D SHALL be listed

#### Scenario: Page edited in an external editor
- **WHEN** the user removes a link from `page.md` with a text editor and then runs `cerbo sparql`
- **THEN** the query SHALL NOT see the removed link

### Requirement: Index failures never block file writes

A failure to update the index SHALL NOT fail or roll back the file operation that triggered it. The system SHALL report the failure as a warning and mark the index stale. The next operation that opens the index SHALL repair it through the catch-up step.

#### Scenario: Busy index during save
- **WHEN** a page is saved while the index cannot be opened
- **THEN** `page.md` and its `.ttl` files SHALL be written successfully
- **THEN** a warning SHALL report that the index is stale

#### Scenario: Stale index is repaired on next use
- **WHEN** the index was marked stale by a failed update and a later command opens it
- **THEN** the missed changes SHALL be applied before that command answers

### Requirement: Single writer with bounded waiting

At most one process SHALL hold the index open at a time, and only for the duration of one operation. A process that finds the index held by another process SHALL retry for a bounded time, then fail with an "index busy" error. It SHALL NOT read the index while another process holds it.

#### Scenario: CLI waits for the desktop app
- **WHEN** the desktop app is updating the index and the user runs `cerbo sparql`
- **THEN** the CLI SHALL wait and run the query once the index is released

#### Scenario: Index stays busy
- **WHEN** the index remains held longer than the retry limit
- **THEN** the command SHALL exit non-zero with an "index busy" message

### Requirement: Full rebuild is atomic

A full rebuild of the index SHALL leave the previous index usable until the new one is complete. An interrupted rebuild SHALL leave either the complete previous index or the complete new index, never a partial one.

#### Scenario: Interrupted rebuild
- **WHEN** a full rebuild is interrupted partway through
- **THEN** the next query SHALL use the previous complete index

### Requirement: Missing or incompatible index is built automatically

If the index is missing, or written in a format version this build does not support, the first operation that needs it SHALL build it from the vault files before continuing, reporting progress. The CLI SHALL report progress on stderr.

#### Scenario: Query before first index
- **WHEN** the user runs `cerbo sparql` in a vault that has never been indexed
- **THEN** the index SHALL be built, with progress on stderr
- **THEN** the query SHALL run against the new index

#### Scenario: Index from an older Cerbo version
- **WHEN** the index format version differs from the one this build writes
- **THEN** the index SHALL be rebuilt before the operation continues

#### Scenario: Vault moved to another path
- **WHEN** the user moves a vault directory and opens it from the new path
- **THEN** a new index SHALL be built for the new path
