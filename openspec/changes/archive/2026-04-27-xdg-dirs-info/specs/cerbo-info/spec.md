# Cerbo Info

## Purpose

Display configuration paths and vault information to help users understand where data is stored.

## ADDED Requirements

### Requirement: CLI info command
The system SHALL provide an `info` subcommand in the CLI that displays configuration paths and registered vaults.

#### Scenario: Run cerbo info with no vaults
- **WHEN** user runs `cerbo info` with no vaults registered
- **THEN** the system SHALL display:
```
Config:  ~/.config/cerbo
Cache:  ~/.cache/cerbo

No vaults registered
```

#### Scenario: Run cerbo info with vaults
- **WHEN** user runs `cerbo info` with vaults registered
- **THEN** the system SHALL display:
```
Config:  ~/.config/cerbo
Cache:  ~/.cache/cerbo

Vaults: 2 registered
├── my-vault (./path/to/vault) - 42 pages
└── work-notes (./notes/work) - 156 pages
```

#### Scenario: Run cerbo info with custom paths
- **WHEN** user has `$XDG_CONFIG_HOME` set to custom location
- **THEN** the system SHALL display the custom path instead of default

### Requirement: Desktop info flag
The desktop application SHALL support an `--info` flag that displays the same information as CLI `info`.

#### Scenario: Desktop --info flag
- **WHEN** user launches `cerbo-desktop --info`
- **THEN** the system SHALL print config/cache paths and vault info to standard output
- **AND** exit immediately without showing the GUI