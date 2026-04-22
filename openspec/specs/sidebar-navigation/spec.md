# Sidebar Navigation

## Purpose
Provide keyboard-driven navigation for the sidebar page list.

## Requirements

### Requirement: Sidebar Arrow Navigation
The system MUST allow users to navigate the page list in the sidebar using arrow keys when a page button has focus or when the sidebar area is active.

#### Scenario: Navigate down with ArrowDown
- **WHEN** focus is on a page item in the sidebar
- **AND** the user presses `ArrowDown`
- **THEN** focus SHALL move to the next page item in the list

#### Scenario: Navigate up with ArrowUp
- **WHEN** focus is on a page item in the sidebar
- **AND** the user presses `ArrowUp`
- **THEN** focus SHALL move to the previous page item in the list

#### Scenario: Wrap navigation at list boundaries
- **WHEN** focus is on the last page item
- **AND** the user presses `ArrowDown`
- **THEN** focus SHALL wrap to the first page item

### Requirement: Sidebar Tab Cycle
The system SHALL allow users to cycle through pages in the sidebar using the `Tab` key.

#### Scenario: Cycle to next page via Tab
- **WHEN** focus is on a page item in the sidebar
- **AND** the user presses `Tab`
- **THEN** focus SHALL move to the next page item
- **AND** the default browser tab behavior SHALL be prevented to keep focus within the list

### Requirement: Vim-style Sidebar Navigation
The system SHALL support `j` and `k` keys for navigating the page list when no input-like element is focused.

#### Scenario: Navigate down with j
- **WHEN** focus is in the sidebar area
- **AND** no input is focused
- **AND** the user presses `j`
- **THEN** focus SHALL move to the next page item

#### Scenario: Navigate up with k
- **WHEN** focus is in the sidebar area
- **AND** no input is focused
- **AND** the user presses `k`
- **THEN** focus SHALL move to the previous page item
