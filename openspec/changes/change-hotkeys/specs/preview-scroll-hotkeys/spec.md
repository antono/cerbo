## ADDED Requirements

### Requirement: Preview Scroll Down
The system SHALL allow users to scroll down in preview mode using the `j` key when no input element is focused.

#### Scenario: Scroll down with j key
- **WHEN** the user is in preview mode and not focusing an input element
- **AND** the user presses `j`
- **THEN** the preview viewport SHALL scroll down by 100 pixels

#### Scenario: Scroll down ignored when input focused
- **WHEN** the user is in preview mode and an input or textarea is focused
- **AND** the user presses `j`
- **THEN** the system SHALL NOT scroll (allow normal input behavior)

### Requirement: Preview Scroll Up
The system SHALL allow users to scroll up in preview mode using the `k` key when no input element is focused.

#### Scenario: Scroll up with k key
- **WHEN** the user is in preview mode and not focusing an input element
- **AND** the user presses `k`
- **THEN** the preview viewport SHALL scroll up by 100 pixels

#### Scenario: Scroll up ignored when input focused
- **WHEN** the user is in preview mode and an input or textarea is focused
- **AND** the user presses `k`
- **THEN** the system SHALL NOT scroll (allow normal input behavior)

### Requirement: Sidebar Next Page
The system SHALL allow users to navigate to the next page in the sidebar using the `J` (uppercase) key when no input element is focused.

#### Scenario: Navigate to next page with J key
- **WHEN** the user is in preview mode and not focusing an input element
- **AND** the user presses `J` (Shift+j)
- **THEN** the system SHALL open the next page in the sidebar list
- **AND** the sidebar focus SHALL move to that page

#### Scenario: Next page ignored when input focused
- **WHEN** the user is in preview mode and an input or textarea is focused
- **AND** the user presses `J`
- **THEN** the system SHALL NOT navigate (allow normal input behavior)

### Requirement: Sidebar Previous Page
The system SHALL allow users to navigate to the previous page in the sidebar using the `K` (uppercase) key when no input element is focused.

#### Scenario: Navigate to previous page with K key
- **WHEN** the user is in preview mode and not focusing an input element
- **AND** the user presses `K` (Shift+k)
- **THEN** the system SHALL open the previous page in the sidebar list
- **AND** the sidebar focus SHALL move to that page

#### Scenario: Previous page ignored when input focused
- **WHEN** the user is in preview mode and an input or textarea is focused
- **AND** the user presses `K`
- **THEN** the system SHALL NOT navigate (allow normal input behavior)
