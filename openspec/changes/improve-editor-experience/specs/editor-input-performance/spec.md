# Spec Delta

## Purpose

Keeps typing in the desktop page editor immediate: a character appears as soon
as the key is pressed, whatever the page size or the plugins loaded.

## ADDED Requirements

### Requirement: Keystroke-to-paint budget
In write mode the editor SHALL paint a typed character within 16 ms (one frame
at 60 Hz) at the 95th percentile, for pages up to 137 KB of markdown, on the
reference developer machine.

#### Scenario: Typing in a large page
- **WHEN** the user types 137 characters in a row into a page of 137 KB
- **THEN** the 95th-percentile time from `keydown` to the next paint is 16 ms or less

#### Scenario: Typing in a small page
- **WHEN** the user types into a page of a few lines
- **THEN** every character is painted in the frame after its `keydown`

### Requirement: Deferred work does not block input
Work that is not needed to show the typed character (syntax overlay, preview
rendering, autosave, link detection) SHALL run off the keystroke path and SHALL
NOT delay the paint of later keystrokes.

#### Scenario: Preview hidden while writing
- **WHEN** the editor is in write mode and the user is typing
- **THEN** the hidden preview is not re-rendered until the user switches to preview or pauses typing

#### Scenario: Autosave during typing
- **WHEN** autosave fires while the user keeps typing
- **THEN** no keystroke is dropped or painted late because of the save
