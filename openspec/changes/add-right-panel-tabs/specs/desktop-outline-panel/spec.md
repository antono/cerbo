# Spec Delta

## Purpose

Shows the headings of the open page as a tree so the user can see its structure and jump to a part.

## ADDED Requirements

### Requirement: Outline lists headings

The Outline sub-panel SHALL list the headings of the open page in document order, indented by level. Headings inside code blocks SHALL NOT be listed. It SHALL show an empty-state message when the page has no headings.

#### Scenario: Headings listed
- **WHEN** the open page has `# Plan`, `## Budget` and `## Risks`
- **THEN** the panel SHALL list Plan with Budget and Risks indented below it

#### Scenario: No headings
- **WHEN** the open page has no headings
- **THEN** the panel SHALL show an empty-state message

### Requirement: Outline follows edits

The list SHALL update while the user types, without waiting for a save.

#### Scenario: New heading appears
- **WHEN** the user types `## Notes` on a new line
- **THEN** Notes SHALL appear in the list

### Requirement: Jump to heading

Clicking a heading SHALL scroll the editor, or the preview in preview mode, so the heading is at the top, and SHALL move the cursor there in edit mode.

#### Scenario: Jump in edit mode
- **WHEN** the user clicks Risks in edit mode
- **THEN** the editor SHALL scroll to the Risks heading and place the cursor on it

### Requirement: Current heading is marked

The heading whose section holds the top of the visible editor area SHALL be marked in the list.

#### Scenario: Marked while scrolling
- **WHEN** the user scrolls the editor into the Budget section
- **THEN** Budget SHALL be marked in the list
