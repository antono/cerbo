# Proposal

## Why

The desktop app runs inside the native window frame of the operating system. The frame duplicates the app's own header, looks different on every desktop environment, and takes vertical space. Cerbo should draw its own window chrome, with the close button inside the app.

## What Changes

- The main window starts without native decorations (`decorations: false`).
- A thin app-drawn title bar runs across the top of the window. It shows the vault and page name. Its empty area drags the window, and double-clicking it maximizes or restores the window.
- A close button sits at the top-right corner of the title bar. It follows exactly the same flow as `Ctrl+Q`: the "Quit Application?" confirmation, then the shared exit path that saves UI settings and window size. There are no minimize or maximize buttons; those stay with the window manager and its shortcuts.
- The window can still be resized from every edge and corner without a native frame. Invisible resize handles start a native resize drag.
- The Tauri capability grants only the window permissions this needs.

## Capabilities

### New Capabilities

- `desktop-window-chrome`: the frameless window, the app-drawn title bar, dragging, maximize on double-click, resize edges, and the close button.

### Modified Capabilities

- `application-lifecycle`: the in-app close button triggers the same confirmed exit as `Ctrl+Q`.

## Non-goals

- Minimize and maximize buttons.
- Changing what the window manager's own close action (for example `Alt+F4`) does.
- A transparent window or rounded window corners.
- Mobile targets.

## Impact

- `src-tauri/tauri.conf.json` (window `decorations`), `src-tauri/capabilities/default.json` (window permissions).
- `src/routes/+layout.svelte` (title bar row, resize handles, close button), a new `src/lib/TitleBar.svelte`, and styles that use the existing design tokens.
- `KeyboardHelp.svelte` is unchanged: no new shortcut.
