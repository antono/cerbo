# Icon System

## Purpose
Establish a unified vector icon system for consistent visual identity and automatic theme support.

## Requirements

### Requirement: Standardized Icon Components
The system SHALL use `lucide-svelte` components for all UI navigation and action icons.

#### Scenario: Icon usage in buttons
- **WHEN** a component renders a UI action (e.g., "Add Vault")
- **THEN** it SHALL use a Lucide icon component with consistent sizing (e.g., 18px or 16px)

### Requirement: Theme-Aware Icon Colors
Icons SHALL automatically adapt their color based on the current theme (light/dark).

#### Scenario: Automatic color inversion
- **WHEN** the user toggles the theme to dark
- **THEN** the icon color SHALL automatically change to match the `--fg` or `--muted-foreground` variables
