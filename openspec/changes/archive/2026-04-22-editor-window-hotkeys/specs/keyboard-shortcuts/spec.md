## ADDED Requirements

### Requirement: Global Page Search Shortcut
The system MUST provide a global keyboard shortcut to open the page autocomplete/search interface from anywhere in the application.

#### Scenario: Open search via Ctrl+P
- **WHEN** the user presses `Ctrl+P` (Linux/Windows) or `Cmd+P` (Mac)
- **THEN** the Page Autocomplete interface SHALL be displayed and focused

### Requirement: Panel Focus Navigation
The system MUST allow users to switch focus between major UI panels using keyboard shortcuts. Major panels include the Page List, the Main Editor, and any active Sidebar panels.

#### Scenario: Switch focus between panels
- **WHEN** the user presses `Ctrl+RightArrow` (Linux/Windows) or `Cmd+RightArrow` (Mac)
- **THEN** the focus SHALL move to the next panel to the right
- **WHEN** the user presses `Ctrl+LeftArrow` (Linux/Windows) or `Cmd+LeftArrow` (Mac)
- **THEN** the focus SHALL move to the previous panel to the left

### Requirement: Cross-Platform Key Mapping
The hotkey system MUST automatically map `Ctrl` (Linux/Windows) to `Cmd` (Mac) for all application-level shortcuts to ensure a native experience on all supported platforms.

#### Scenario: Mapping shortcuts on Mac
- **WHEN** the application is running on macOS
- **THEN** all defined `Ctrl` shortcuts MUST be triggered by the `Cmd` key instead
