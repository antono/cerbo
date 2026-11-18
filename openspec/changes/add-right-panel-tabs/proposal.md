## Why

The right panel stacks Backlinks, Similar notes and Attachments in one column, and has no room for more. An agent chat (see `add-agent-panel`) and an outline are coming, so the panel needs to hold many sub-panels and show one at a time.

## What Changes

- Replace the stacked right panel with a strip of small icon tabs along its top edge. Each tab opens one sub-panel; only one shows at a time.
- Tabs: **Attachments**, **Links** (backlinks, outgoing links, similar notes), **Outline** (headings of the open page) and **Agent** (a placeholder here; filled by `add-agent-panel`).
- Remember the last chosen tab across restarts and keep it when the open page changes.
- Add an Outline sub-panel: the open page's headings as a tree; clicking one scrolls the editor to it.
- Add outgoing links to the Links sub-panel next to the existing backlinks and similar notes.
- Narrow layout: the bottom sheet uses the same tabs instead of its own Backlinks/Similar switch.
- Each tab has a hotkey and a command-palette action.

## Capabilities

### New Capabilities
- `desktop-right-panel`: the tab strip, tab selection and persistence, hotkeys, and how the panel behaves in wide, medium and narrow layouts.
- `desktop-outline-panel`: the Outline sub-panel.

### Modified Capabilities
- `desktop-similar-notes-panel`: Similar notes moves from its own section under Backlinks into the Links sub-panel.

## Impact

- Frontend: `src/lib/RightSidebarPanel.svelte` (becomes the tab host), `AttachmentsPanel.svelte`, `SimilarNotesSection.svelte`, `PanelOverlay.svelte`, `src/routes/+page.svelte`, `layout.ts`, `actions.ts`, `hotkeys.ts`, UI-settings persistence.
- Backend: one new read for outgoing links, if the index does not already expose it.
- No change to the vault format or the CLI.
