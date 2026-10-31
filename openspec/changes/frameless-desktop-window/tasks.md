# Tasks

## 1. Window and permissions

- [ ] 1.1 Set `decorations: false` for the main window in `src-tauri/tauri.conf.json`; verify the built app opens without a native frame (`nix build .#cerbo-desktop`)
- [x] 1.2 Add `core:window:allow-start-dragging`, `core:window:allow-start-resize-dragging` and `core:window:allow-toggle-maximize` to `src-tauri/capabilities/default.json`; verify the build regenerates the capability schema without errors
- [ ] 1.3 Check whether undecorated windows already resize natively from their edges on the platforms at hand (GNOME/KDE, X11/Wayland); record the result in design.md D4
  - Source review recorded in D4: the Tauri runtime resizes undecorated windows natively on Linux (GTK, X11 and Wayland), Windows and macOS. Still open: confirm live on GNOME and KDE under X11 and Wayland during 3.2.

## 2. Title bar and close button

- [x] 2.1 Add `src/lib/TitleBar.svelte` (drag region, "Vault · Page" title, close button with `aria-label="Quit Cerbo"` that sets `app.showExitPrompt = true`) and mount it above `app-shell` in `+layout.svelte`; adjust layout heights; verify `bun run check` and a vitest test that the close button opens the exit prompt state
- [ ] 2.2 Add the eight resize handles from design D4 (hidden when maximized), unless 1.3 shows native resizing works everywhere; verify `bun run check`
  - Not added: the D4 source review shows native resizing on every platform. Tick or implement after the live check in 1.3/3.2.

- [x] 2.3 Draw a 1px `--window-border` outline on `body` with square corners, and remove it while the window is maximized or fullscreen (`window-maximized` class set from `isMaximized`/`isFullscreen` on every resize); verify `bun run check`, `bunx vitest run` and `nix build .#cerbo-desktop`

## 3. Checks

- [x] 3.1 Run `bun run check`, `bunx vitest run` (inside `nix develop`), `nix build .#cerbo-desktop` and `openspec validate frameless-desktop-window --strict`; verify all pass
- [ ] 3.2 Manual pass on the built app in light and dark themes: drag, double-click maximize, resize from each edge and corner, close button → confirmation → Cancel and Quit, window size restored on restart
