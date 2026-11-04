# Spec Delta

## ADDED Requirements

### Requirement: Command-line page move
The `cerbo` CLI SHALL provide `cerbo page move <UUID> <PATH> [--json]` to set a page's `cerbo:virtualPath`, and `cerbo page move <UUID> --root [--json]` to clear it. `PATH` and `--root` are mutually exclusive and one is required. On success the command prints `Moved <uuid> to <path>` (or `to root`); with `--json`, `{"uuid": ..., "virtualPath": <string or null>}`. Validation failures and unknown UUIDs exit non-zero with the reason on stderr.

#### Scenario: Move a page from the CLI
- **WHEN** user runs `cerbo page move <uuid> notes/rust`
- **THEN** the page's `meta.ttl` contains `cerbo:virtualPath "notes/rust"`
- **THEN** stdout is `Moved <uuid> to notes/rust`

#### Scenario: Move to root with --json
- **WHEN** user runs `cerbo page move <uuid> --root --json`
- **THEN** the `cerbo:virtualPath` predicate is removed
- **THEN** stdout is only `{"uuid": "<uuid>", "virtualPath": null}`

#### Scenario: Invalid path
- **WHEN** user runs `cerbo page move <uuid> notes/`
- **THEN** the command exits non-zero
- **THEN** stderr names the validation rule and `meta.ttl` is unchanged

#### Scenario: Both PATH and --root
- **WHEN** user runs `cerbo page move <uuid> notes --root`
- **THEN** the command exits non-zero with a clap usage error

#### Scenario: Unknown UUID
- **WHEN** user runs `cerbo page move 00000000-0000-0000-0000-000000000000 notes`
- **THEN** the command exits non-zero with "Object not found" on stderr

### Requirement: Command-line folder rename
The `cerbo` CLI SHALL provide `cerbo folder rename <OLD> <NEW> [--json]` to replace the prefix `OLD` with `NEW` on every page whose path is `OLD` or starts with `OLD/`. On success it prints `Renamed <old> to <new> (<n> pages)`; with `--json`, `{"from": ..., "to": ..., "pages": <n>}`.

#### Scenario: Rename a folder from the CLI
- **WHEN** pages exist at `notes`, `notes/rust` and `other`
- **AND** user runs `cerbo folder rename notes archive`
- **THEN** those pages now have paths `archive`, `archive/rust` and `other`
- **THEN** stdout is `Renamed notes to archive (2 pages)`

#### Scenario: Folder rename with no matching pages
- **WHEN** user runs `cerbo folder rename nothing here`
- **THEN** the command exits zero and reports `0 pages`

#### Scenario: Invalid new prefix
- **WHEN** user runs `cerbo folder rename notes "/archive"`
- **THEN** the command exits non-zero before writing any page

### Requirement: Page list reports virtual paths
`cerbo page list` SHALL show each page's virtual path, and `cerbo page list --json` SHALL include a `virtualPath` field (string or null) in every entry.

#### Scenario: Text listing shows the path
- **WHEN** a page has `cerbo:virtualPath "notes/rust"`
- **THEN** `cerbo page list` prints its line as `<uuid>: <title> [notes/rust]`
- **THEN** a page with no path prints as `<uuid>: <title>` with no suffix

#### Scenario: JSON listing includes virtualPath
- **WHEN** user runs `cerbo page list --json`
- **THEN** each object has `uuid`, `title` and `virtualPath`
