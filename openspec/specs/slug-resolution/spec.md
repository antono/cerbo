# Slug Resolution

## Purpose
Define the standard for converting page titles to filesystem-safe slugs and resolving wikilinks.

## Requirements

### Requirement: Derive slug from title
The system SHALL derive a deterministic kebab-case slug from a page title using the following algorithm: (1) Unicode NFKD normalize, (2) strip combining diacritics, (3) lowercase, (4) replace runs of non-alphanumeric characters with a single hyphen, (5) trim leading and trailing hyphens.

#### Scenario: ASCII title
- **WHEN** the title is "Rust Ownership"
- **THEN** the slug is `rust-ownership`

#### Scenario: Title with diacritics
- **WHEN** the title is "café & résumé"
- **THEN** the slug is `cafe-resume`

#### Scenario: Title with special characters
- **WHEN** the title is "C++ Basics"
- **THEN** the slug is `c-basics`

#### Scenario: Title with leading/trailing punctuation
- **WHEN** the title is "!My Tauri App!"
- **THEN** the slug is `my-tauri-app`

### Requirement: Resolve wikilink to page
The system SHALL resolve a `[[Title]]` wikilink to a page by normalizing the link text using the slug algorithm and looking it up in the vault's page index. Resolution SHALL be case-insensitive.

#### Scenario: Exact title match
- **WHEN** the wikilink text is "Rust Ownership" and a page with slug `rust-ownership` exists
- **THEN** the link resolves to `rust-ownership/page.md`

#### Scenario: Lowercase link text
- **WHEN** the wikilink text is "rust ownership"
- **THEN** the link resolves to `rust-ownership/page.md`

#### Scenario: Direct slug as link text
- **WHEN** the wikilink text is "rust-ownership"
- **THEN** the link resolves to `rust-ownership/page.md`

#### Scenario: No matching page
- **WHEN** the wikilink text does not match any page slug in the vault
- **THEN** the link is marked as broken and the user is offered the option to create the page

### Requirement: Expose slug function to frontend
The system SHALL expose the slug derivation function as a Tauri command so the frontend can show the derived slug in real time during page creation.

#### Scenario: Frontend previews slug
- **WHEN** the user types a page title in the creation UI
- **THEN** the frontend calls the slug command and displays the derived slug beneath the title field
