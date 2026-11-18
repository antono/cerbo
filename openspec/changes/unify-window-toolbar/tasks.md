# Tasks

## 1. Action registry and settings

- [x] 1.1 Add `left_sidebar_visible: Option<bool>` to `core/src/ui_settings.rs`, following `right_sidebar_visible`. Verify with a round-trip unit test and with a settings file that lacks the field (`cargo test -p cerbo-core ui_settings`).
- [x] 1.2 Add `src/lib/actions.ts`: `{ id, label, icon, hotkey?, synonyms?, when?, run }` for every action in `desktop-window-toolbar` and `command-palette`. Verify with `actions.test.ts`:
  - every hotkey is unique;
  - every action with `when` hides when no page is open;
  - `Ctrl+\` matches on `e.code`.
- [x] 1.3 Replace the hand-written `if` chain in `+layout.svelte` with registry matching. Add `Ctrl+\`, `Ctrl+Shift+\` and `Ctrl+E`. Verify:
  - existing hotkeys behave the same in the e2e suite (`e2e/run.sh`);
  - a unit test covers the new keys, including while the editor is focused.
- [x] 1.4 Render the global section of `KeyboardHelp.svelte` from the registry. Verify the help lists `Ctrl+\`, `Ctrl+Shift+\`, `Ctrl+E` and `Ctrl+Shift+P` (spec `hotkeys-help`).

## 2. Window toolbar

- [ ] 2.1 Create `WindowToolbar.svelte`, set `data-tauri-drag-region="deep"` on it, and remove `TitleBar.svelte`. Verify:
  - drag and double-click maximize work in the built app;
  - `scripts/screenshot.sh` shows one bar.
- [x] 2.2 Add the vault control, which opens the existing selector or reads "Add vault" with no vault. Remove the sidebar vault header. Update `vault-open.e2e.ts` selectors and verify it passes.
- [x] 2.3 Add the path with folder segments and the page menu (Rename, Move, Copy link, Reveal in tree, Delete), using `RowMenu`. Verify with an e2e test:
  - rename from the page menu works;
  - a folder segment reveals that folder.
- [x] 2.4 Move the save state into the toolbar as a dot after the title, and remove the floating "Saving…" label. Verify `autosave-leave.e2e.ts` passes with an updated selector.
- [ ] 2.5 Add the search field, new page, mode toggle and both panel toggles with pressed states.
  - Remove the "Pages" header, the editor mode button, the Carta toolbar height override in `app.css`, the floating "Show panels" button and the right panel ✕.
  - Verify `attachment-drop.e2e.ts` and `typing-latency.e2e.ts` pass, and that the toggles persist across a restart.
- [x] 2.6 Add the ⋯ menu (theme, SPARQL console, New folder, Keyboard shortcuts, Quit) and remove the sidebar footer. Verify with e2e that:
  - the theme entry label flips;
  - "Quit" shows the confirmation (spec `application-lifecycle`).
- [x] 2.7 Add tooltips with hotkeys and accessible names to every toolbar control. Verify with an e2e check that each button has an accessible name.

## 3. Command mode

- [x] 3.1 Add the `>` mode to `GlobalSearch.svelte`: actions from the registry, substring match on label and synonyms, recently run first. Verify with unit tests for matching, `when` filtering and the "Dark theme" synonym case.
- [x] 3.2 Make `Ctrl+Shift+P` open search with `>` and toggle it closed. Make Enter close search and then run the action. Verify with an e2e test that `>folder` + Enter opens the New Folder dialog with focus.

## 4. Responsive layout

- [ ] 4.1 Add `app.widthClass` from a `ResizeObserver` (1000 / 700), and set `minWidth: 360` and `minHeight: 480` in `src-tauri/tauri.conf.json`. Verify with a unit test of the class function and by resizing the built app below the minimum.
- [x] 4.2 Medium layout:
  - the right panel opens as an overlay;
  - search collapses to an icon;
  - the path collapses to "… / Title" with a segment list.
  - Verify with `scripts/screenshot.sh -s 800x600 -c …` that the editor width is unchanged when the panel opens.
- [x] 4.3 Narrow layout:
  - the tree opens as a drawer with the vault menu and new page;
  - the right panel opens as a bottom sheet with tabs (the `variant` prop on the panels);
  - the toolbar is reduced and the mode button floats.
  - Verify with `scripts/screenshot.sh -s 390x844` that the page fills the width, and with an e2e test that picking a page in the drawer closes it.
- [x] 4.4 Overlay rules: `Esc`, outside click and the toggle close an overlay; opening one closes the other; overlays do not change the stored docked state. Verify with an e2e test on a 390px window and with a unit test of the state transitions.

## 5. Integration

- [x] 5.1 Run the full gate and verify every command passes:
  - `nix build .#cerbo .#cerbo-desktop`
  - `bun test`
  - `e2e/run.sh`
- [x] 5.2 Take screenshots at 1200x760, 1000x700, 800x600 and 390x844, and check them against the mockups agreed in the session.

## Workflow follow-up

- Archive the change after review.
