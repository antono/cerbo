# Spec Delta

## MODIFIED Requirements

### Requirement: Sidebar Arrow Navigation
The system MUST allow users to navigate the sidebar tree using arrow keys when a row has focus or when the sidebar area is active. Navigation moves over the visible rows only: folder rows and page rows in display order, skipping rows hidden inside collapsed folders.

#### Scenario: Navigate down with ArrowDown
- **WHEN** focus is on a row in the sidebar
- **AND** the user presses `ArrowDown`
- **THEN** focus SHALL move to the next visible row, whether it is a folder or a page

#### Scenario: Navigate up with ArrowUp
- **WHEN** focus is on a row in the sidebar
- **AND** the user presses `ArrowUp`
- **THEN** focus SHALL move to the previous visible row

#### Scenario: Wrap navigation at list boundaries
- **WHEN** focus is on the last visible row
- **AND** the user presses `ArrowDown`
- **THEN** focus SHALL wrap to the first visible row

#### Scenario: Collapsed rows are skipped
- **WHEN** the folder "notes" is collapsed and focus is on its row
- **AND** the user presses `ArrowDown`
- **THEN** focus SHALL move to the row after "notes" at the same or a higher level, not to a page inside it

### Requirement: Sidebar Tab Cycle
The system SHALL allow users to cycle through visible rows in the sidebar using the `Tab` key.

#### Scenario: Cycle to next page via Tab
- **WHEN** focus is on a row in the sidebar
- **AND** the user presses `Tab`
- **THEN** focus SHALL move to the next visible row
- **AND** the default browser tab behavior SHALL be prevented to keep focus within the list

### Requirement: Vim-style Sidebar Navigation
The system SHALL support `j` and `k` keys for moving over visible rows, and `h` and `l` for collapsing and expanding folders, when no input-like element is focused.

#### Scenario: Navigate down with j
- **WHEN** focus is in the sidebar area
- **AND** no input is focused
- **AND** the user presses `j`
- **THEN** focus SHALL move to the next visible row

#### Scenario: Navigate up with k
- **WHEN** focus is in the sidebar area
- **AND** no input is focused
- **AND** the user presses `k`
- **THEN** focus SHALL move to the previous visible row

#### Scenario: Collapse with h
- **WHEN** focus is on an expanded folder row
- **AND** the user presses `h`
- **THEN** the folder SHALL collapse and keep focus

#### Scenario: h on a page or collapsed folder goes to the parent
- **WHEN** focus is on a page row or a collapsed folder row that has a parent folder
- **AND** the user presses `h`
- **THEN** focus SHALL move to the parent folder row

#### Scenario: Expand with l
- **WHEN** focus is on a collapsed folder row
- **AND** the user presses `l`
- **THEN** the folder SHALL expand and keep focus

#### Scenario: l on an expanded folder enters it
- **WHEN** focus is on an expanded folder row
- **AND** the user presses `l`
- **THEN** focus SHALL move to the first row inside it

## ADDED Requirements

### Requirement: Folder rows respond to Enter, Space and arrows
A focused folder row SHALL toggle on `Enter` or `Space`. `ArrowLeft` SHALL behave as `h` and `ArrowRight` as `l`. A focused page row SHALL open the page on `Enter`.

#### Scenario: Toggle folder with Enter
- **WHEN** focus is on a folder row
- **AND** the user presses `Enter`
- **THEN** the folder's expanded state SHALL flip

#### Scenario: ArrowRight expands
- **WHEN** focus is on a collapsed folder row
- **AND** the user presses `ArrowRight`
- **THEN** the folder SHALL expand

#### Scenario: Open page with Enter
- **WHEN** focus is on a page row
- **AND** the user presses `Enter`
- **THEN** that page SHALL open in the editor

### Requirement: Move shortcut in the sidebar
Pressing `m` while a page row is focused and no input is focused SHALL open the Move dialog for that page. Pressing `m` on a folder row SHALL open the Rename folder dialog for that folder.

#### Scenario: m on a page
- **WHEN** focus is on a page row
- **AND** the user presses `m`
- **THEN** the Move dialog opens prefilled with that page's current path

#### Scenario: m on a folder
- **WHEN** focus is on a folder row
- **AND** the user presses `m`
- **THEN** the Rename folder dialog opens for that folder

#### Scenario: m is ignored while typing
- **WHEN** an input is focused
- **AND** the user presses `m`
- **THEN** the key is inserted as text and no dialog opens
