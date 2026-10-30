# Spec Delta

## Purpose

Shows, next to the open page, the notes closest to it in meaning, so related thoughts surface while writing without leaving the editor.

## ADDED Requirements

### Requirement: Similar notes section in the right sidebar

The right sidebar SHALL show a "Similar notes" section below Backlinks. For the open page it SHALL list up to 10 other pages closest in meaning, best first, each with its title and similarity score as a percentage. Clicking an entry SHALL open that page by UUID. The open page SHALL never appear in its own list.

#### Scenario: Related notes are listed
- **WHEN** the user opens a page about a trip to Berlin and the vault has embeddings
- **THEN** the section SHALL list pages closest in meaning with titles and scores
- **THEN** clicking an entry SHALL call `openPage(entry.uuid)`

#### Scenario: Section updates on navigation
- **WHEN** the user opens another page
- **THEN** the section SHALL show the similar notes of the newly opened page

### Requirement: Embeddings not enabled

When the vault has no embeddings yet, the section SHALL explain that semantic search is off and offer an "Enable" action. The action SHALL compute embeddings for the vault (the same work as `cerbo index --embeddings`), show progress, including the one-time model download, and then load the list.

#### Scenario: Enabling from the panel
- **WHEN** the vault has no embeddings and the user clicks "Enable"
- **THEN** the section SHALL show progress until embeddings are computed
- **THEN** the similar notes of the open page SHALL appear without reopening it

#### Scenario: Model cannot be obtained
- **WHEN** the model is not cached and cannot be downloaded
- **THEN** the section SHALL show that the embedding model is unavailable and SHALL offer to retry

### Requirement: The panel never blocks editing

Loading similar notes, refreshing embeddings and downloading the model SHALL run in the background. While they run, the page SHALL stay editable and the section SHALL show a loading state. If the index is busy, the section SHALL show that similar notes are temporarily unavailable.

#### Scenario: Typing during a refresh
- **WHEN** embeddings are being refreshed and the user types in the editor
- **THEN** input SHALL NOT be delayed by the refresh

#### Scenario: Navigating away during loading
- **WHEN** the user opens page B while page A's similar notes are still loading
- **THEN** the section SHALL show B's results only, never A's

### Requirement: Empty result

When no other page is similar enough, the section SHALL show an empty-state message instead of a list.

#### Scenario: Single-page vault
- **WHEN** the vault contains only the open page
- **THEN** the section SHALL show an empty-state message
