# Keyboard Shortcuts

## Purpose
Provide standard and efficient keyboard-driven navigation and control across the application.

## Requirements

### Requirement: Global Page Search Shortcut
The system MUST provide a global keyboard shortcut to open the page autocomplete/search interface from anywhere in the application.

#### Scenario: Open search via Ctrl+P
- **WHEN** the user presses `Ctrl+P` (Linux/Windows) or `Cmd+P` (Mac)
- **THEN** the Page Autocomplete interface SHALL be displayed and focused

### Requirement: Create Page Shortcut
The system MUST provide a global keyboard shortcut to open the "New Page" form from anywhere in the application.

#### Scenario: Open new page form via Ctrl+N
- **WHEN** the user presses `Ctrl+N` (Linux/Windows) or `Cmd+N` (Mac)
- **THEN** the New Page form SHALL be displayed and focused
- **THEN** any other active dialogs or forms (Search, Vault Switcher) SHALL be automatically closed

### Requirement: Escape to Close
The system MUST use the `Esc` key as a universal "close" or "cancel" action for all transient UI elements, including modals, forms, and the vault switcher.

#### Scenario: Close active form via Esc
- **WHEN** any transient UI element is active (New Page form, Search modal, Vault Switcher, Rename form)
- **AND** the user presses `Esc`
- **THEN** the active element SHALL be closed and focus returned to the previous context

### Requirement: Panel Focus Navigation
The system MUST allow users to switch focus between major UI panels using keyboard shortcuts. Major panels include the Page List, the Main Editor, and any active Sidebar panels.

#### Scenario: Switch focus between panels
- **WHEN** the user presses `Ctrl+RightArrow` (Linux/Windows) or `Cmd+RightArrow` (Mac)
- **THEN** the focus SHALL move to the next panel to the right
- **WHEN** the user presses `Ctrl+LeftArrow" (Linux/Windows) or `Cmd+LeftArrow" (Mac)
- **THEN** the focus SHALL move to the previous panel to the left

### Requirement: Cross-Platform Key Mapping
The hotkey system MUST automatically map `Ctrl` (Linux/Windows) to `Cmd` (Mac) for all application-level shortcuts to ensure a native experience on all supported platforms.

#### Scenario: Mapping shortcuts on Mac
- **WHEN** the application is running on macOS
- **THEN** all defined `Ctrl` shortcuts MUST be triggered by the `Cmd` key instead
