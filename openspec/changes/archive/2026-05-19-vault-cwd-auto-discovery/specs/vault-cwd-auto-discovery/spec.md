## ADDED Requirements

### Requirement: CWD vault auto-discovery
The system SHALL walk up from the current working directory at CLI startup to find the nearest ancestor directory containing `.cerbo/`. If found, that directory SHALL be used as the effective vault root for the invocation, unless `--vault` is explicitly provided. The walk SHALL use the same mount-point-aware helper defined in the `vault-management` capability.

#### Scenario: CWD is inside a registered vault
- **WHEN** the user runs any cerbo command from inside a directory tree rooted at a registered vault
- **THEN** the command operates on that vault without requiring `--vault`

#### Scenario: CWD is inside a vault subdirectory (not the root)
- **WHEN** the user's CWD is several levels below the vault root (e.g. `<vault>/notes/projects/`)
- **THEN** the walk-up locates the vault root and the command operates on that vault

#### Scenario: Explicit --vault overrides CWD discovery
- **WHEN** the user provides `--vault <id>` on any command
- **THEN** the explicitly provided vault ID SHALL be used regardless of CWD
- **THEN** CWD discovery is not performed

#### Scenario: CWD vault not in registry
- **WHEN** the walk-up finds a `.cerbo/` at an ancestor but no registered vault matches that path
- **THEN** the system SHALL emit a warning to stderr: `warning: vault at <path> is not registered; run 'cerbo vault add'`
- **THEN** the system SHALL fall through to the active-vault state and single-vault fallback

#### Scenario: CWD not inside any vault
- **WHEN** no `.cerbo/` is found in CWD or any ancestor up to the mount-point boundary
- **THEN** CWD discovery yields no vault
- **THEN** the system falls through to the active-vault state, then single-vault fallback, then error

### Requirement: Path-to-ID matching uses canonical paths
The system SHALL canonicalize both the discovered vault root path and the registered vault paths before comparing them, so that symlinks and redundant path components do not cause false mismatches.

#### Scenario: Discovered path matches registered path via symlink
- **WHEN** the vault is registered under `/real/path` and the user is inside `/link/path` (a symlink to `/real/path`)
- **THEN** canonicalization resolves both to the same real path and the vault is matched

### Requirement: Effective vault resolution priority
The system SHALL resolve the effective vault for each command invocation using the following priority order (highest to lowest):

1. Explicit `--vault <id>` flag
2. CWD-discovered vault (walk-up, path matched to registry)
3. Active vault from persisted state (`cerbo vault active`)
4. Single registered vault (when only one vault is registered)
5. Error

#### Scenario: Only active vault set, not in CWD
- **WHEN** CWD is not inside any vault and an active vault is set in state
- **THEN** the active vault is used

#### Scenario: Multiple vaults registered, none active, CWD not inside any
- **WHEN** multiple vaults are registered, no active vault is set, and CWD is not inside any vault
- **THEN** the command fails with an error indicating no vault could be determined
