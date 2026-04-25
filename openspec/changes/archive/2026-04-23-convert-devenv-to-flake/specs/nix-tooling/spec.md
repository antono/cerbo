## ADDED Requirements

### Requirement: Nix Development Shell
The system SHALL provide a Nix development shell that includes all tools necessary to build and run Cerbo.

#### Scenario: Enter development environment
- **WHEN** user runs `nix develop` in the project root
- **THEN** a shell is provided containing `rustc`, `cargo`, `bun`, and `cargo-tauri`.

### Requirement: Nix Package for Cerbo CLI
The system SHALL provide a Nix package definition for the `cerbo` CLI.

#### Scenario: Build Cerbo CLI with Nix
- **WHEN** user runs `nix build .#cerbo`
- **THEN** a binary named `cerbo` is produced in `./result/bin/`.

### Requirement: Nix Package for Cerbo Desktop
The system SHALL provide a Nix package definition for the `cerbo-desktop` application.

#### Scenario: Build Cerbo Desktop with Nix
- **WHEN** user runs `nix build .#cerbo-desktop`
- **THEN** a Tauri-based desktop application is produced in `./result/bin/`.
