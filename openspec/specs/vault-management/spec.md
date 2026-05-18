# Vault Management

## Purpose
Manage vault initialization and registration using `cerbo init` command.
## Requirements
### Requirement: Initialize vault
The system SHALL provide `cerbo init` command that initializes a new vault in the current directory. The command SHALL create `.cerbo/` directory with `objects/`, `index.json`, and `ontology-map.json`. The command SHALL create bundled ontology objects (Schema.org, FOAF). The command SHALL ensure that the vault root contains a `.gitignore` file with a line of exactly `/cerbo/`, so the materialised symlink tree (see the `vault-symlink` capability) is never committed.

#### Scenario: Initialize new vault
- **WHEN** user runs `cerbo init` in an empty directory
- **THEN** `.cerbo/` directory is created
- **THEN** `.cerbo/objects/` directory is created
- **THEN** `.cerbo/index.json` is created with empty title_to_uuid and uuid_to_path
- **THEN** `.cerbo/ontology-map.json` is created
- **THEN** Schema.org ontology object is created in `.cerbo/objects/<uuid>/`
- **THEN** FOAF ontology object is created in `.cerbo/objects/<uuid>/`
- **THEN** `.gitignore` is created at the vault root containing the line `/cerbo/`

#### Scenario: Re-run init on existing vault
- **WHEN** user runs `cerbo init` on a directory with existing `.cerbo/`
- **THEN** the command SHALL succeed without error
- **THEN** existing objects SHALL NOT be deleted
- **THEN** existing `ontology-map.json` SHALL NOT be overwritten

#### Scenario: Append `/cerbo/` to existing `.gitignore`
- **WHEN** user runs `cerbo init` in a directory where `.gitignore` exists and does NOT already contain a line exactly matching `/cerbo/`
- **THEN** the command appends a section to `.gitignore` containing a one-line comment (e.g. `# Cerbo symlink tree (regenerate with: cerbo symlink)`) followed by the line `/cerbo/`
- **THEN** prior `.gitignore` contents SHALL NOT be modified or reordered

#### Scenario: `.gitignore` already has the entry
- **WHEN** user runs `cerbo init` in a directory where `.gitignore` already contains a line exactly matching `/cerbo/`
- **THEN** `.gitignore` SHALL NOT be modified

### Requirement: Vault is directory with .cerbo/
The system SHALL treat any directory containing `.cerbo/` as a valid vault (synonymously, a **cerbo vault**). The vault root is the parent directory of `.cerbo/`.

#### Scenario: Detect vault
- **WHEN** the system scans for vaults
- **THEN** it looks for directories containing `.cerbo/` subdirectory
- **THEN** the vault name is derived from the directory name (or from `meta.ttl` if available)

#### Scenario: Vault and cerbo vault are synonyms
- **WHEN** documentation or user-facing messages refer to a "cerbo vault"
- **THEN** the term refers to the same concept as "vault" — a directory containing `.cerbo/`

### Requirement: Repository discovery by walk-up

The system SHALL provide a public helper that, given a starting directory, walks upward through ancestor directories until it finds one containing a `.cerbo/` subdirectory; the first such directory found SHALL be returned as the vault root. The walk SHALL stop at filesystem mount-point boundaries (detected via `stat::st_dev` changing between a directory and its parent) and at the filesystem root. If no `.cerbo/` is found, the helper SHALL return None, and callers SHALL surface a user-facing error of the form `not a cerbo vault (or any parent up to mount point)`. This helper SHALL be reusable across all cerbo commands that operate on the current repository.

#### Scenario: Repository at starting directory
- **WHEN** the starting directory itself contains a `.cerbo/` subdirectory
- **THEN** the helper returns that directory as the vault root

#### Scenario: Repository in an ancestor
- **WHEN** the starting directory is several levels below a directory containing `.cerbo/`
- **THEN** the helper walks up and returns the nearest ancestor containing `.cerbo/`

#### Scenario: No repository found
- **WHEN** no `.cerbo/` exists in the starting directory or any ancestor up to the filesystem root or a mount-point boundary
- **THEN** the helper returns None
- **THEN** callers SHALL exit non-zero with `not a cerbo vault (or any parent up to mount point)` on stderr

#### Scenario: Mount-point boundary stops the walk
- **WHEN** the walk would cross a filesystem mount-point boundary before finding `.cerbo/`
- **THEN** the helper stops at the boundary and returns None

### Requirement: CLI dispatch uses walk-up helper as default vault resolver
The CLI dispatch layer SHALL invoke the `vault-management` walk-up helper at startup (before command dispatch) and use the result as the default vault for all commands that operate on vault content. This requirement formalises the wiring between the existing walk-up helper and the CLI layer.

#### Scenario: Walk-up result passed to page commands
- **WHEN** the user runs `cerbo page list` (or any page subcommand) from inside a vault
- **THEN** the command operates on the vault discovered from CWD, not the global XDG config

#### Scenario: Walk-up result passed to resolve command
- **WHEN** the user runs `cerbo resolve <uuid>` from inside a vault
- **THEN** the command looks up the UUID within the CWD-discovered vault

#### Scenario: Walk-up result passed to import commands
- **WHEN** the user runs `cerbo import <url>` or `cerbo import-ontology <url>` from inside a vault
- **THEN** the imported object is stored in the CWD-discovered vault

### Requirement: vault_id_from_path helper
The system SHALL expose a public function `vault_id_from_path(ctx, root: &Path) -> Option<String>` in `cerbo_core::vault` that scans the registered vaults list and returns the ID of the first vault whose canonical path equals the canonical form of `root`.

#### Scenario: Match found
- **WHEN** `root` canonicalizes to the same path as a registered vault
- **THEN** the function returns `Some(vault_id)`

#### Scenario: No match
- **WHEN** no registered vault's canonical path equals the canonical form of `root`
- **THEN** the function returns `None`
