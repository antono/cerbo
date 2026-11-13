# Spec Delta

## Purpose

Rewrites the metadata of existing vaults from Cerbo's legacy private RDF spellings into the standard vocabulary, so old vaults match what new writes produce.

## ADDED Requirements

### Requirement: Migrate vault metadata to the standard vocabulary

`cerbo migrate vocabulary` SHALL rewrite every live object's `meta.ttl` and `annotations.ttl` that uses a legacy form into the standard form. Legacy forms are the terms replaced by the standard vocabulary (`cerbo:type`, the classes `cerbo:Product`/`Source`/`Attachment`/`Ontology`, `cerbo:title`, `cerbo:original-url`, `cerbo:mime-type`, `schema:` bound to `cerbo://ontology/schema/`) and annotations as `cerbo:annotation`/`cerbo:concept`. Other triples SHALL be preserved.

#### Scenario: Legacy meta.ttl is rewritten
- **WHEN** an object's `meta.ttl` contains `cerbo:type cerbo:Product`, `cerbo:title "Index"` and `<cerbo://ontology/schema/dateCreated>`
- **AND** the user runs `cerbo migrate vocabulary`
- **THEN** the file SHALL contain `rdf:type <https://schema.org/NoteDigitalDocument>` and no `cerbo:Product`, plus `<https://schema.org/name> "Index"` and `<https://schema.org/dateCreated>`
- **THEN** the title, slug, virtual path and date values SHALL be unchanged

#### Scenario: Legacy annotation type is rewritten
- **WHEN** a page's `annotations.ttl` types an annotation as `<cerbo://objects/<uuid-schema-org>/Place>` and the map maps `schema` to that ontology object
- **AND** the user runs `cerbo migrate vocabulary`
- **THEN** the page SHALL `schema:mentions` a node typed `<https://schema.org/Place>` with `schema:name` equal to the former `cerbo:concept`
- **THEN** no `cerbo:annotation` or `cerbo:concept` triple SHALL remain

#### Scenario: Annotation whose prefix was lost
- **WHEN** an annotation type is `<cerbo://ontology/Place>` and the page text still contains `[Berlin]{schema:Place}`
- **THEN** the migration SHALL re-derive the type from the page text as `<https://schema.org/Place>`

### Requirement: Migration scope and safety

The command SHALL accept `--vault <id>` to limit the migration to one vault, and SHALL otherwise migrate every registered vault. It SHALL skip trashed objects. Each file SHALL be rewritten with the atomic replacement protocol. An object that cannot be read SHALL be reported with its UUID, SHALL be left untouched, and SHALL make the command exit non-zero after the other objects are processed.

#### Scenario: One vault only
- **WHEN** the user runs `cerbo migrate vocabulary --vault <id>`
- **THEN** only objects of that vault SHALL be rewritten

#### Scenario: Unreadable object
- **WHEN** one of 137 objects has a `meta.ttl` that does not parse
- **THEN** the other 136 objects SHALL be migrated
- **THEN** stderr SHALL name the broken object's UUID
- **THEN** the command SHALL exit non-zero

### Requirement: Dry run

With `--dry-run`, the command SHALL report what it would change and SHALL NOT modify any file.

#### Scenario: Dry run changes nothing
- **WHEN** the user runs `cerbo migrate vocabulary --dry-run` on a legacy vault
- **THEN** no file under `.cerbo/` SHALL be modified
- **THEN** the output SHALL list the number of files that would be rewritten

### Requirement: Migration is idempotent

Running the migration on a vault that is already in the standard form SHALL rewrite no file and SHALL exit zero.

#### Scenario: Second run
- **WHEN** the user runs `cerbo migrate vocabulary` twice
- **THEN** the second run SHALL report zero rewritten files
- **THEN** no file modification time SHALL change during the second run

### Requirement: Migration summary

The command SHALL print a summary with the number of objects scanned, files rewritten, and errors. With `--json` it SHALL print only a JSON object with the numeric fields `scanned`, `rewritten` and `errors`.

#### Scenario: JSON summary
- **WHEN** the user runs `cerbo migrate vocabulary --json`
- **THEN** stdout SHALL be a valid JSON object with numeric `scanned`, `rewritten` and `errors`

### Requirement: CLI detects and migrates the old format automatically

Before a `cerbo` command works on a vault, the CLI SHALL find the live objects of that vault that `cerbo migrate vocabulary` would rewrite, on every such command, so legacy files that arrive later are found. A command that writes to the vault SHALL migrate exactly those objects first, printing to stderr `Warning: <N> objects in vault "<name or path>" use the old metadata format; migrating them to schema.org`, the migration's notes, each failed object's UUID and reason, and one result line.

#### Scenario: Writing command migrates with a warning
- **WHEN** a registered vault named "Legacy" holds one object with a legacy `meta.ttl` and `annotations.ttl`
- **AND** the user runs `cerbo page create New --json` inside it
- **THEN** stderr SHALL contain `Warning: 1 objects in vault "Legacy" use the old metadata format; migrating them to schema.org`
- **THEN** both files SHALL be in the standard form
- **THEN** the page SHALL be created as usual

#### Scenario: Migration notes are shown
- **WHEN** the automatic migration keeps an annotation type as `cerbo:unresolvedType`
- **THEN** stderr SHALL contain the same warning `cerbo migrate vocabulary` prints for it

#### Scenario: JSON output stays clean
- **WHEN** the user runs `cerbo page list --json` or `cerbo index --json` on a legacy vault
- **THEN** stdout SHALL be only the command's valid JSON
- **THEN** the warning (and, for `index`, the result line) SHALL be on stderr

#### Scenario: Current vault
- **WHEN** every object of the vault is in the standard form
- **THEN** no warning SHALL be printed and no file under `.cerbo/` SHALL change

#### Scenario: File synced from an old device later
- **WHEN** a vault was already migrated
- **AND** sync then replaces one `meta.ttl` with a legacy one and adds a new object with a legacy `meta.ttl`
- **THEN** the next command SHALL warn about 2 objects and migrate both

#### Scenario: Unchanged vault is not rescanned
- **WHEN** the check has found every object of a vault of 137 objects current
- **AND** no file of the vault changed since
- **THEN** the next check SHALL read no object file
- **WHEN** one page's `page.md` is then edited
- **THEN** the next check SHALL read only that object

#### Scenario: Failing migration does not block the command
- **WHEN** a legacy object's directory cannot be written
- **AND** the user runs `cerbo page create <title> --json`
- **THEN** stderr SHALL name the object's UUID and the reason it could not be migrated
- **THEN** the command SHALL succeed, and the page SHALL still be listed, read in the old format

#### Scenario: Dry run is not preceded by a migration
- **WHEN** the user runs `cerbo migrate vocabulary --dry-run` inside a legacy vault
- **THEN** no file under `.cerbo/` SHALL be modified

### Requirement: Automatic migration scope

`page list`, `page read`, `page backlinks`, `page create`, `page write`, `page delete`, `resolve`, `sparql`, `search`, `similar`, `import`, `import-ontology`, `index` and `symlink` SHALL migrate, on every vault the command works on, resolved the way the command resolves it; `index` SHALL migrate before the index reads the vault. Nothing SHALL run for `init`, `vault`, `info`, `--help`, `--version` or `cerbo migrate vocabulary`.

#### Scenario: Reading command migrates too
- **WHEN** a registered vault named "Legacy" holds one object with a legacy `meta.ttl`
- **AND** the user runs `cerbo page list --json` inside it
- **THEN** stderr SHALL contain `Warning: 1 objects in vault "Legacy" use the old metadata format; migrating them to schema.org`
- **THEN** the object SHALL be migrated and the page list printed as usual

#### Scenario: Index sees migrated files
- **WHEN** the user runs `cerbo index --vault <path> --json` on a legacy vault
- **THEN** the files SHALL be migrated before the index is rebuilt

### Requirement: Automatic migration never rewrites under a running app

A running Cerbo desktop app SHALL hold the app lock `$XDG_CACHE_HOME/cerbo/desktop.lock` shared for its whole life, and every migration SHALL write only while holding it exclusively, taken without waiting. When it is held, the automatic migration SHALL write nothing and SHALL print `Warning: <N> objects in vault "<name or path>" use the old metadata format; not migrating them while Cerbo desktop or another migration is running`.

#### Scenario: App running during a writing command
- **WHEN** a Cerbo desktop app is running
- **AND** the user runs `cerbo page create <title>` in a legacy vault
- **THEN** the legacy object's files SHALL NOT change
- **THEN** stderr SHALL say that the migration is deferred

### Requirement: Explicit migration refuses while the app runs

`cerbo migrate vocabulary` without `--dry-run` SHALL fail with an error and write nothing when it cannot take the app lock exclusively. `--dry-run` SHALL run regardless.

#### Scenario: Migrate with the app open
- **WHEN** a Cerbo desktop app is running
- **AND** the user runs `cerbo migrate vocabulary`
- **THEN** the exit status SHALL be non-zero and no vault file SHALL change

### Requirement: Migration keeps files saved meanwhile

Every migration SHALL leave a file alone, and report its object as an error, when the file's content changed between being read and being rewritten.

#### Scenario: App saves while a migration runs
- **WHEN** a legacy `meta.ttl` is saved by another writer after the migration read it and before it is rewritten
- **THEN** the migration SHALL keep the saved file and report the object

### Requirement: Automatic migration check is cached

The check SHALL keep, per vault, the `(mtime_ns, size)` of the files of objects already found current in `$XDG_CACHE_HOME/cerbo/vocabulary/<vault key>-<rules stamp>`, SHALL read only objects whose files changed since, and SHALL write nothing into the vault. The rules stamp SHALL change with the Cerbo version and with the legacy rules, so a table written under other rules is never trusted.

#### Scenario: Check writes nothing into the vault
- **WHEN** the check runs on a legacy vault
- **THEN** no file under `.cerbo/` SHALL change until the migration itself runs

#### Scenario: New legacy rules re-read cached objects
- **WHEN** a build with other legacy rules cached an object as current
- **AND** that object's unchanged file uses a spelling the current rules count as legacy
- **THEN** the next check SHALL read the object and report it

### Requirement: Automatic migration never blocks a command

A vault that cannot be checked, an object that cannot be migrated, a cache that cannot be written, or an app lock that cannot be taken SHALL be reported on stderr as a warning, and the command SHALL still run. Nothing SHALL be written to stdout.

#### Scenario: Unwritable cache
- **WHEN** the cache directory cannot be created
- **THEN** the legacy objects SHALL still be found, with a warning, and the command SHALL succeed

### Requirement: Migrate only what is needed

`cerbo migrate vocabulary --if-needed` SHALL run the automatic check and migration on its target vaults (every registered vault by default) and SHALL print nothing on stdout without `--json`. It SHALL NOT be combined with `--dry-run`. No `cerbo migrate` command SHALL register the vault found from the current directory.

#### Scenario: Nothing left to migrate
- **WHEN** every registered vault is current
- **AND** the user runs `cerbo migrate vocabulary --if-needed`
- **THEN** stdout and stderr SHALL be empty and the exit status zero

#### Scenario: Started from inside an unregistered vault
- **WHEN** the current directory is inside a vault that is not registered
- **AND** `cerbo migrate vocabulary --if-needed` runs there
- **THEN** that vault SHALL NOT be registered and none of its files SHALL change

### Requirement: Desktop launcher migrates vaults before start

The packaged `cerbo-desktop` launcher SHALL run `cerbo migrate vocabulary --if-needed` on every registered vault before it starts the app, on every start. Its output SHALL go to stderr. A failure of that command SHALL NOT keep the app from starting. The launcher SHALL keep the program name `cerbo-desktop`, so the `.desktop` entry and the window still match. The package build SHALL check the generated launcher.

#### Scenario: Legacy vault at start
- **WHEN** a registered vault holds legacy files
- **AND** no other Cerbo desktop app is running
- **AND** the user starts `cerbo-desktop`
- **THEN** the files SHALL be in the standard form before the app process starts
- **THEN** the migration's warning SHALL be on stderr

#### Scenario: Second start while the app runs
- **WHEN** a Cerbo desktop app is already running
- **AND** the user starts `cerbo-desktop` again
- **THEN** no vault file SHALL be rewritten before the new app starts

#### Scenario: Failing migration at start
- **WHEN** `cerbo migrate vocabulary --if-needed` exits non-zero
- **THEN** the launcher SHALL still start the app with its arguments
