# Spec: Cerbo Link Resolution

On user click, resolve `cerbo://objects/<uuid>` links to their current page paths and navigate to the resolved destination.

## ADDED Requirements

### Requirement: Cerbo URL rendering and click handling
Cerbo:// links SHALL render with semantic href and resolve on click via the `cerbo resolve` mechanism.

#### Scenario: Cerbo link renders with semantic href
- **WHEN** rendered page contains `[Page Name](cerbo://objects/<valid-uuid>)`
- **THEN** the link renders as `<a href="cerbo://objects/<uuid>">Page Name</a>`
- **AND** the href is visible/semantic (not transformed)
- **AND** the link is clickable

#### Scenario: Valid UUID resolves on click
- **WHEN** user clicks a cerbo:// link with a valid UUID
- **THEN** the click handler calls `cerbo resolve <uuid>` to get the current path
- **AND** navigation happens to the resolved path
- **AND** page loads and displays correctly

#### Scenario: Page renamed, link still works on click
- **WHEN** a linked page is renamed and user clicks the cerbo:// link
- **THEN** `cerbo resolve` returns the new path of the renamed page
- **AND** clicking still navigates to the correct (renamed) page
- **AND** the link text in brackets (`[Page Name]`) may be stale, but navigation works

#### Scenario: Invalid or missing UUID
- **WHEN** user clicks a cerbo:// link with a non-existent UUID
- **THEN** `cerbo resolve` returns an error or empty result
- **AND** user sees an error message (e.g., "Page not found" dialog)
- **AND** rendering is not affected (page continues to display)
- **AND** user can dismiss error and continue editing/browsing

#### Scenario: Resolve handles vault context
- **WHEN** user clicks a cerbo:// link in a vault
- **THEN** UUID resolution is scoped to the current vault
- **AND** links do not accidentally resolve to objects in other vaults
- **AND** if UUID exists in different vault, error is shown with explanation

### Requirement: Click handler attachment
Click handlers for cerbo:// links SHALL be reliably attached during page rendering and work in both desktop and web contexts.

#### Scenario: Click handler initialized
- **WHEN** page renders with cerbo:// links
- **THEN** all cerbo:// links have click handlers attached
- **AND** clicking triggers resolution and navigation
- **AND** no console errors from handler initialization

#### Scenario: Handler works in desktop app
- **WHEN** user clicks cerbo:// link in desktop app (Tauri)
- **THEN** navigation happens within the app
- **AND** resolved path is correctly interpreted for app context

#### Scenario: Handler works in web context
- **WHEN** user clicks cerbo:// link in web editor
- **THEN** navigation happens via app router (e.g., SvelteKit)
- **AND** resolved path is interpreted as relative/absolute route

### Requirement: Link text preservation
Link text in `[...]` portion of cerbo:// links is preserved during rendering and may diverge from actual page title.

#### Scenario: Stale link text after page rename
- **WHEN** page "Project A" is renamed to "Project Alpha" and had incoming links with old text
- **THEN** rendered links still display the old text `[Project A]` (stale)
- **AND** navigation still works to the renamed page (via UUID)
- **AND** user sees stale text until source is manually updated

#### Scenario: Custom link text
- **WHEN** user writes `[Custom Label](cerbo://objects/<uuid>)`
- **THEN** renders as `<a href="/path/to/uuid">Custom Label</a>`
- **AND** custom text is preserved even if it doesn't match the page title
