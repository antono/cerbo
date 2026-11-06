# Spec Delta

## ADDED Requirements

### Requirement: Vault path shown in the selector
Each vault row in the vault selector SHALL show the vault's full absolute
filesystem path next to its name. A path too long for the row SHALL be shortened
in the middle with an ellipsis, and the full path SHALL be available on hover.

#### Scenario: Name and path side by side
- **WHEN** the user opens the vault selector
- **THEN** each row shows the vault name and, beside it in secondary text, its absolute path (e.g. `/home/user/notes/work`)

#### Scenario: Same name, different places
- **WHEN** two registered vaults are both named `Notes`
- **THEN** the rows are told apart by their paths

#### Scenario: Long path
- **WHEN** a vault path does not fit in the row
- **THEN** the middle of the path is replaced with `…`, keeping its start and the vault folder name visible
- **AND** hovering the row shows the full path
