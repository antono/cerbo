# Proposal

## Why

The desktop window stacks up to three horizontal bars before any content: the
title bar, the vault header, and the "Pages" header over the tree. The editor has
its own 48px bar that holds a single button. Window actions are spread over six
places, and the vault name and page title are each shown twice. Two identical ✕
buttons do different things: the top one quits the app, the right-panel one hides
the panel. The SPARQL console, history back/forward and "Add vault" are
keyboard-only, and nothing on screen points to them. Below 700px the editor
disappears completely: at 390px only the tree and a clipped right panel are left
(checked with `scripts/screenshot.sh -s 390x844`).

The goal is to reach every feature with one click or one hotkey, from one place,
at any window width.

## What Changes

- One window toolbar replaces the title bar, vault header, "Pages" header,
  editor mode bar, sidebar footer, the floating "Show panels" button and the
  right panel's ✕. **BREAKING** (UI): these elements are removed.
- Toolbar contents, left to right:
  - vault menu ▾
  - left panel toggle
  - back / forward
  - path of the open page, with a page menu ▾ (rename, move, copy link,
    reveal in tree, delete)
  - search field
  - new page
  - edit / preview toggle
  - right panel toggle
  - ⋯ menu (theme, SPARQL console, new folder, shortcuts, quit)
  - close button
- The empty toolbar area keeps dragging the window and toggling maximize on
  double-click.
- Page search gets a command mode. Typing `>` lists every app action with its
  hotkey, and `Ctrl+Shift+P` opens search straight in that mode.
- New hotkeys:
  - `Ctrl+\` shows or hides the page tree.
  - `Ctrl+Shift+\` shows or hides the right panel.
  - `Ctrl+E` switches between edit and preview.
- Layout adapts to window width:
  - **Wide:** all three columns side by side.
  - **Medium:** the right panel floats over the editor instead of squeezing it.
  - **Narrow (phone width):** one column. The tree and the right panel open as
    overlays, and the toolbar collapses to ☰, ←, page ▾, search and ⋯.
- `scripts/screenshot.sh` (already added) is the way to check each layout.

## Non-goals

- No mobile (Android/iOS) build target. "Phone width" means a narrow desktop
  window, for example a tiling layout or a split screen.
- No minimize or maximize buttons. The close button stays the only window
  control.
- No changes to the right panel's content (backlinks, similar notes,
  attachments) beyond how it is shown and hidden.
- No user-configurable toolbar or hotkeys.

## Capabilities

### New Capabilities
- `desktop-window-toolbar`: the single toolbar, its controls, the vault, page
  and ⋯ menus, and its panel toggles.
- `desktop-responsive-layout`: how the tree, editor and right panel are laid
  out at wide, medium and narrow window widths.
- `command-palette`: the command mode of page search that lists and runs app
  actions.

### Modified Capabilities
- `desktop-window-chrome`: the title bar becomes the window toolbar. The page
  title is no longer repeated next to the vault name.
- `vault-selector`: the selector opens from the toolbar's vault menu, not from a
  sidebar header.
- `hotkeys-help`: help opens from the ⋯ menu instead of an icon next to the
  theme toggle.
- `theme-toggle-component`: the theme switch moves into the ⋯ menu.
- `keyboard-shortcuts`: the new panel, mode and command-mode hotkeys.
- `application-lifecycle`: the quit path is triggered from the toolbar's close
  button.

## Impact

- Frontend:
  - `src/routes/+layout.svelte`
  - `src/routes/+page.svelte`
  - `src/lib/TitleBar.svelte` (replaced by the toolbar)
  - `src/lib/PageList.svelte` (header removed)
  - `src/lib/PageEditor.svelte` (mode button removed)
  - `src/lib/RightSidebarPanel.svelte`
  - `src/lib/GlobalSearch.svelte` (command mode)
  - `src/lib/KeyboardHelp.svelte`
  - `src/lib/stores.svelte.ts` (left panel visibility and width class)
  - `src/app.css` (Carta toolbar override)
- `src-tauri/tauri.conf.json`: a minimum window width.
- e2e specs that click the removed controls need new selectors.
- `core/src/ui_settings.rs` gains one optional field, `left_sidebar_visible`,
  following `right_sidebar_visible`. Old settings files keep loading.
- No CLI changes.
