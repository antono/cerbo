## 1. Tab host

- [x] 1.1 Add `right-panels.ts` registry (id, label, icon, hotkey, badge) with tests
- [x] 1.2 Add `rightPanelTab` to UI settings in `core/src/ui_settings.rs` (optional field, merge, fallback) with tests
- [x] 1.3 Replace `RightSidebarPanel.svelte` with the tab host: icon strip, selected state, tooltips, accessible names
- [x] 1.4 Keep sub-panels mounted and hide the unselected; gate loading on a `visible` prop; keep scroll position
- [x] 1.5 Persist and restore the selected tab; keep it across page changes

## 2. Sub-panels

- [x] 2.1 Move Attachments into the host; show the attachment count badge
- [x] 2.2 Add `LinksPanel`: Backlinks, Outgoing links, Similar notes sections
- [x] 2.3 Add `outgoing_links_get` Tauri command (reuse the core index query if present)
- [x] 2.4 Add `extractHeadings` with tests (levels, fenced code, empty page)
- [x] 2.5 Add `OutlinePanel`: live list, jump to heading, mark current heading
- [x] 2.6 Add the Agent tab placeholder with a "not connected" status mark

## 3. Layouts, hotkeys, commands

- [x] 3.1 Use the host in the docked panel, the medium floating panel and the narrow sheet; remove `variant='tabs'` and the sheet's own tab bar
- [x] 3.2 Add a hotkey and a "Show <tab>" command per tab (open if hidden, toggle if selected); list them in the help overlay

## 4. Verification

- [x] 4.1 Update existing e2e tests for the new layout and add e2e for tab switching, persistence and the narrow sheet
- [x] 4.2 Run vitest, cargo tests and `nix build .#cerbo .#cerbo-desktop`
