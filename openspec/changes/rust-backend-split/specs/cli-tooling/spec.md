## ADDED Requirements

### Requirement: Command-line page creation
The `cerbo` CLI SHALL allow creating a new page in a specific vault.

#### Scenario: Create a page from CLI
- **WHEN** user runs `cerbo page create --vault <id> --title "My New Page"`
- **THEN** a new folder with a slugified title is created and contains `page.md` with the title as an H1.

### Requirement: Command-line page rename
The `cerbo` CLI SHALL allow renaming a page and triggering a cascade.

#### Scenario: Rename a page from CLI
- **WHEN** user runs `cerbo page rename --vault <id> --slug <old-slug> --title "New Title"`
- **THEN** the page is renamed and all links in other pages are updated.

### Requirement: Command-line vault management
The `cerbo` CLI SHALL allow listing, adding, and removing vaults.

#### Scenario: List vaults from CLI
- **WHEN** user runs `cerbo vault list`
- **THEN** the system prints a list of all registered vaults from the shared `vaults.json` registry.
