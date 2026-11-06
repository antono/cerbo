# Spec Delta

## Purpose

Brings the user back to where they stopped: entering a page's editor restores
the caret and scroll position from the end of the previous editing session on
that page.

## ADDED Requirements

### Requirement: Remember the last position per page
The system SHALL remember, per vault and page, the caret position (line and
column) and scroll position of the write view at the moment an editing session
on that page ends. A session ends on switching to preview, opening another page,
switching vaults, or closing the app.

#### Scenario: Leave to another page
- **WHEN** the user edits page A with the caret on line 137 and opens page B
- **THEN** line 137 and its column are stored for page A

#### Scenario: Close the app
- **WHEN** the user closes the app while editing a page
- **THEN** the caret and scroll of that page are stored before the window closes

### Requirement: Restore the position on entering the editor
Entering write mode on a page SHALL place the caret at the stored position and
scroll so that the caret line is visible at its stored scroll offset, including
after an app restart.

#### Scenario: Come back to a page
- **WHEN** the user returns to page A and enters write mode
- **THEN** the caret is on line 137 at the stored column and that line is in view

#### Scenario: After restart
- **WHEN** the app is restarted and the user enters write mode on page A
- **THEN** the caret is at the position stored before the restart

### Requirement: Fallback when the stored position is unusable
When no position is stored, or the stored line no longer exists, the caret SHALL
go to the start of the first body line after the title, or to the end of the
page if the stored line is past the end.

#### Scenario: Never edited
- **WHEN** the user enters write mode on a page with no stored position
- **THEN** the caret is at the start of line 2

#### Scenario: Page shortened externally
- **WHEN** the stored line is 137 but the page now has 40 lines
- **THEN** the caret is at the end of the page
