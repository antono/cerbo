## ADDED Requirements

### Requirement: Persist editor cursor position on preview switch
The system MUST save the current cursor position for the active editor when the user switches from edit mode to preview mode.

#### Scenario: Save cursor position when leaving edit mode
- **WHEN** the user exits edit mode for preview
- **THEN** the system saves the current cursor position for the active editor

### Requirement: Restore saved cursor position on editor entry
The system MUST restore the previously saved cursor position when the user returns to edit mode for the same editor.

#### Scenario: Restore cursor on re-entering editor
- **WHEN** the user returns to edit mode for an editor with saved cursor metadata
- **THEN** the system places the cursor at the saved position

### Requirement: Scroll to restored cursor
The system MUST scroll the editor after restoration so the cursor is visible in the viewport.

#### Scenario: Make restored cursor visible
- **WHEN** the system restores a cursor position
- **THEN** the editor scrolls so the cursor is visible

### Requirement: Fallback to line 2 for invalid cursor positions
The system MUST place the cursor on line 2 when the saved cursor position is invalid or beyond the current file length.

#### Scenario: Recover from stale cursor metadata
- **WHEN** the saved cursor position is outside the current file range
- **THEN** the system places the cursor on line 2
