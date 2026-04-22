# Background Sync

## Purpose
Enable headless monitoring of vaults to keep the index up to date without the GUI running.

## Requirements

### Requirement: Headless watching
The `cerbo` CLI SHALL provide a `watch` command that stays alive and monitors vault changes.

#### Scenario: Start watcher
- **WHEN** user runs `cerbo watch`
- **THEN** the process monitors all registered vaults for changes to `page.md` files and updates the index accordingly.
- **THEN** the process continues to run until terminated by the user.
