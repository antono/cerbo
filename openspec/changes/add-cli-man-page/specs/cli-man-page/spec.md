## ADDED Requirements

### Requirement: Man page source in Markdown format
The system SHALL maintain the man page source in Markdown format at `cli/man/cerbo.md`.

#### Scenario: Markdown source file exists
- **WHEN** the cli crate is built
- **THEN** the file `cli/man/cerbo.md` exists and is valid Markdown

#### Scenario: Markdown contains conventional man page sections
- **WHEN** the Markdown source is inspected
- **THEN** it SHALL contain these sections: NAME, SYNOPSIS, DESCRIPTION, COMMANDS, OPTIONS, STORAGE, EXAMPLES, SEE ALSO

### Requirement: Markdown to troff conversion using mandown
The build system SHALL use the `mandown` Rust crate to convert Markdown to troff/groff format.

#### Scenario: build.rs converts Markdown to troff
- **WHEN** the cli crate is built
- **THEN** `build.rs` SHALL use `mandown::render_manpage()` to convert `cli/man/cerbo.md` to troff format

#### Scenario: Generated man page output location
- **WHEN** the conversion is complete
- **THEN** the troff output SHALL be written to `$OUT_DIR/cerbo.1`

#### Scenario: Rebuild only on source changes
- **WHEN** `cli/man/cerbo.md` has not changed
- **THEN** the build SHALL NOT regenerate the man page (use `cargo:rerun-if-changed`)

### Requirement: Man page follows conventional format
The generated man page SHALL follow standard Unix man page conventions.

#### Scenario: Man page has correct header
- **WHEN** the troff output is inspected
- **THEN** it SHALL start with `.TH CERBO 1 "date" "cerbo" "User Commands"` format

#### Scenario: Man page sections are properly formatted
- **WHEN** the troff output is inspected
- **THEN** sections SHALL use `.SH` macros (e.g., `.SH NAME`, `.SH SYNOPSIS`)

#### Scenario: Commands section documents all CLI commands
- **WHEN** the man page is viewed with `man cerbo`
- **THEN** all available CLI commands SHALL be listed with descriptions

### Requirement: Commands and arguments are documented
The man page SHALL explain each CLI command, its arguments, and usage.

#### Scenario: vault command documented
- **WHEN** the man page is viewed
- **THEN** the `vault` command SHALL be documented with subcommands (`list`, `add`, `remove`) and their arguments

#### Scenario: Each command has argument descriptions
- **WHEN** a command with arguments is documented
- **THEN** each argument SHALL have a clear description of its purpose and expected values

### Requirement: Storage model described
The man page SHALL include a brief description of the UUID-based storage model.

#### Scenario: STORAGE section exists
- **WHEN** the man page is viewed
- **THEN** a `STORAGE` section SHALL describe the `.cerbo/` directory structure and UUID-based object storage

#### Scenario: Storage description covers key concepts
- **WHEN** the STORAGE section is read
- **THEN** it SHALL mention: Vaults, Objects (with UUIDs), Pages (page.md + meta.ttl), Links (cerbo://uuid format), and Annotations

### Requirement: Man page installs with CLI tool
The generated man page SHALL be installed alongside the `cerbo` binary.

#### Scenario: Man page in Nix package
- **WHEN** the Nix package `cerbo` is installed
- **THEN** the man page SHALL be installed to `$out/share/man/man1/cerbo.1`

#### Scenario: Man page accessible via man command
- **WHEN** a user runs `man cerbo`
- **THEN** the cerbo man page SHALL be displayed

### Requirement: CLI API changes reflected in man page
Any change to the CLI API (commands, arguments, options) SHALL be reflected in the man page.

#### Scenario: New command added to CLI
- **WHEN** a new command is added to the CLI
- **THEN** the man page SHALL be updated to document the new command before the change is considered complete

#### Scenario: Command argument modified
- **WHEN** a command's arguments are modified (added, removed, changed)
- **THEN** the man page SHALL be updated to reflect the changes
