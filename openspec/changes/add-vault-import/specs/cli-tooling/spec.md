# Spec Delta

## ADDED Requirements

### Requirement: Command-line vault import
The `cerbo` CLI SHALL provide `cerbo import-vault <DIR> [--from
obsidian|tolaria] [--vault <ID> | --new <PATH> [--name <NAME>]] [--force]
[--dry-run] [--json]`, behaving as the `vault-import` capability describes,
and the man page SHALL document it. `cerbo import <URL>` SHALL keep its
current behaviour.

#### Scenario: Help lists the command
- **WHEN** user runs `cerbo --help`
- **THEN** `import-vault` is listed next to `import` and `import-ontology`

#### Scenario: Man page section
- **WHEN** user runs `man cerbo`
- **THEN** an `import-vault` section describes every option and gives an Obsidian and a Tolaria example

#### Scenario: Conflicting target flags
- **WHEN** user passes both `--vault` and `--new`
- **THEN** the command exits non-zero with a usage error and writes nothing

#### Scenario: URL import unchanged
- **WHEN** user runs `cerbo import https://example.com/article`
- **THEN** a Source object is created as before
