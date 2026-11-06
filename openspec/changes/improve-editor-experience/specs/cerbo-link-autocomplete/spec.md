# Spec Delta

## ADDED Requirements

### Requirement: Single bracket opens the link picker
Typing `[` in write mode SHALL open a link picker dropdown at the caret listing
vault objects with type and title. The existing `[[` and `(cerbo://` triggers
SHALL keep working.

#### Scenario: Open picker with one bracket
- **WHEN** the user types `[` in ordinary text
- **THEN** a dropdown opens below the caret with a search field and the first objects of the vault, sorted by title

#### Scenario: Task list checkbox is not a link
- **WHEN** the user types `[` right after a list marker (`- `, `* `, `+ `, `1. `) at the start of a line
- **THEN** the picker does not open

#### Scenario: Code is not a link
- **WHEN** the caret is inside an inline code span or a fenced code block
- **THEN** typing `[` does not open the picker

### Requirement: Instant search in the link picker
The picker SHALL contain a search field showing the text typed after `[`. The
list SHALL re-filter on every keystroke, before the next paint, matching title,
object type and virtual path case-insensitively. The editor caret SHALL stay in
the page while the user types the query.

#### Scenario: Filter as you type
- **WHEN** the picker is open and the user types `ber` after `[`
- **THEN** the search field shows `ber`
- **AND** the list shows only objects whose title, type or virtual path contains `ber`, prefix matches first

#### Scenario: Filtering a large vault
- **WHEN** the vault holds 13 700 objects and the user types a query character
- **THEN** the filtered list is shown in the same frame as the typed character

#### Scenario: No matches
- **WHEN** no object matches the query
- **THEN** the picker shows "No objects found" and stays open until dismissed

### Requirement: Picker selection and dismissal
Choosing an object SHALL replace `[` plus the query with
`[<Title>](cerbo://objects/<uuid>)` and put the caret after `)`. Dismissing SHALL
leave the typed `[` and query as plain text.

#### Scenario: Select with keyboard
- **WHEN** the user moves with ArrowUp/ArrowDown and presses Enter or Tab
- **THEN** `[` and the query are replaced with `[Title](cerbo://objects/<uuid>)`
- **AND** the caret is placed after the closing `)`

#### Scenario: Select with mouse
- **WHEN** the user clicks an entry
- **THEN** the same insertion happens and focus returns to the editor

#### Scenario: Dismiss and keep plain text
- **WHEN** the user presses Escape, types `]`, or types a space right after `[`
- **THEN** the picker closes and the typed text stays as written
