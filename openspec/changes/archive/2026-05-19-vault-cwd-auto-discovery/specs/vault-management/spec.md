## ADDED Requirements

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
