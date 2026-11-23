## ADDED Requirements

### Requirement: Import dialog offers Obsidian first
The desktop app SHALL provide an "Import vault…" action that opens a dialog
listing the supported source apps with Obsidian first and Tolaria second.

#### Scenario: Source order
- **WHEN** the dialog opens
- **THEN** the first source card is Obsidian and the second is Tolaria

### Requirement: Preview before writing
The dialog SHALL show the dry-run report for the chosen folder and target
before any file is written, and SHALL write only after the user confirms.

#### Scenario: Preview writes nothing
- **WHEN** the user reaches the preview step
- **THEN** the target vault and the source folder are unchanged

### Requirement: Format mismatch is surfaced
If detection finds a different format than the one chosen, the dialog SHALL
say so and offer to switch; it SHALL NOT import as the wrong format silently.

#### Scenario: Tolaria folder under Obsidian
- **WHEN** the user picks a Tolaria folder with Obsidian selected
- **THEN** the dialog offers to switch to Tolaria

### Requirement: Repeat import needs confirmation
A repeat import of the same folder into the same vault SHALL be refused with
the core message unless the user ticks "Import again anyway".

#### Scenario: Repeat refused
- **WHEN** the folder was already imported into the chosen vault
- **THEN** Import is blocked and the checkbox is offered
