# UI State

## ADDED Requirements

### Requirement: UI settings stored in ui.toml
The system SHALL store persistent UI preferences in `$XDG_CONFIG_DIR/cerbo/ui.toml`.

#### Scenario: First run - no ui.toml
- **WHEN** ui.toml does not exist
- **THEN** system SHALL use default UI settings (system theme, default font size)

#### Scenario: Save UI setting
- **WHEN** user changes sidebar width
- **THEN** value persisted to ui.toml and persists across restarts

### Requirement: UI settings include theme
The system SHALL store user's theme preference in ui.toml.

#### Scenario: Theme persisted
- **WHEN** user selects light theme
- **THEN** theme stored in ui.toml

### Requirement: UI settings include font size
The system SHALL store user's font size preference in ui.toml.

#### Scenario: Font size persisted
- **WHEN** user adjusts font size
- **THEN** font size stored in ui.toml

### Requirement: UI settings include sidebar width
The system SHALL store user's sidebar width in ui.toml.

#### Scenario: Sidebar width persisted
- **WHEN** user resizes sidebar
- **THEN** width stored in ui.toml

## ADDED Requirements

### Requirement: Ephemeral state in state.toml
The system SHALL store transient UI state in `$XDG_CACHE_DIR/cerbo/state.toml`.

#### Scenario: Active vault stored in state
- **WHEN** user selects active vault
- **THEN** vault id stored in state.toml (NOT config.toml)

#### Scenario: Last open page per vault
- **WHEN** user opens a page
- **THEN** page path stored in state.toml under vault's key

#### Scenario: State not persisted to config
- **WHEN** system writes active vault state
- **THEN** it writes to state.toml, NOT config.toml