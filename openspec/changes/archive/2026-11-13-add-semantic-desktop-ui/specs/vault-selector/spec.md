# Spec Delta

## ADDED Requirements

### Requirement: Vault entries show the Cerbo icon

Each vault entry in the vault selector SHALL show the Cerbo app icon (the brain-on-chip design, the same artwork as the app and window icon), scaled down to the entry's line height, before the vault name. The icon SHALL be decorative: it SHALL NOT change the entry's accessible name. It SHALL stay legible in both light and dark themes.

#### Scenario: Every vault has the icon
- **WHEN** the user opens the vault selector with three registered vaults
- **THEN** each of the three entries SHALL show the Cerbo icon before its name

#### Scenario: Icon matches the app icon
- **WHEN** the app icon artwork changes in `src/favicon.svg`
- **THEN** the vault selector SHALL show the new artwork without a separate copy to update

#### Scenario: Screen readers read only the name
- **WHEN** a screen reader focuses a vault entry named "Notes"
- **THEN** it SHALL announce "Notes" and nothing for the icon

#### Scenario: Dark theme
- **WHEN** the app uses the dark theme
- **THEN** the icon's outline SHALL remain distinguishable from the selector background
