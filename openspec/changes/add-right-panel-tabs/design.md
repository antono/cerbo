# Design

## Context

`RightSidebarPanel.svelte` stacks Backlinks and Similar notes; `AttachmentsPanel.svelte` sits beneath it in `+page.svelte`. A narrow-layout variant (`variant='tabs'`) already switches between Backlinks and Similar through a tab bar owned by `PanelOverlay.svelte`. Layout classes and overlays are pure functions in `layout.ts`. See proposal.md for motivation.

## Goals / Non-Goals

**Goals:**
- One tab host that every layout uses.
- Adding a sub-panel later means registering one entry, not editing layout code.

**Non-Goals:**
- Splitting the panel into several visible sub-panels at once.
- Drag-reordering or hiding tabs.
- Any agent behaviour (`add-agent-panel`).

## Decisions

**D1. A registry of panels.** `right-panels.ts` exports an ordered list of `{ id, label, icon, hotkey, component, badge? }`. The tab strip and the command-palette actions are generated from it. Alternative: hard-code four buttons. Rejected: the Agent tab and later ones would touch the host every time.

**D2. The host replaces `RightSidebarPanel`.** It renders the strip and mounts the selected component. The old Backlinks and Similar code moves into a `LinksPanel` that composes the existing sections. The narrow `variant='tabs'` prop is removed, and `PanelOverlay` stops owning a tab bar.

**D3. Keep sub-panels mounted, hide the unselected.** Mounted-but-hidden keeps scroll position and avoids reloading. Cost: hidden panels must not do work. Links and Outline load only while selected (a `visible` prop gates their effects); Agent keeps its connection alive regardless, since that lives outside the component.

**D4. Selected tab in UI settings.** A new `rightPanelTab` string next to `rightSidebarVisible`, with the same optional-field merge behaviour in `core/src/ui_settings.rs`. Unknown values fall back to `links`.

**D5. Outline from the editor, not from disk.** Headings come from the live editor document so they update while typing. Pure `extractHeadings(markdown)` (skips fenced code) is unit-tested; the current-heading mark uses the editor's scroll position.

**D6. Outgoing links.** Taken from the same semantic index as backlinks through `cerbo-core`, via a new `outgoing_links_get` command mirroring `backlinks_get`. If core already has the query, only the Tauri command is new.

## Risks / Trade-offs

- [Hidden panels still mounted cost memory] → gate effects on `visible`; panels are small lists.
- [Hotkey clashes] → pick hotkeys through `hotkeys.ts` conflict checks and show them in the help overlay.
- [Removing `variant='tabs'` breaks narrow e2e tests] → update the e2e specs in the same task as the layout change.

## Migration Plan

Pure UI. A saved settings file without `rightPanelTab` loads as Links. Rolling back means reverting the change; the extra field is ignored by older builds.
