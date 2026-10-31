# Design

## Context

- `src-tauri/tauri.conf.json` declares one window ("Cerbo", 800×600) with native decorations.
- The capability `src-tauri/capabilities/default.json` grants `core:default` plus theme permissions. `core:window:default` already includes `allow-is-maximized` and `allow-internal-toggle-maximize`.
- Exit flow: `Ctrl+Q` → `ExitConfirmation.svelte` → `quitApp()` in `stores.svelte.ts` → `saveUiSettings()` + the Rust command `app_exit`. Window size is persisted through `window_bounds_save`.
- The layout is `app-shell` = sidebar + resize handle + main. The right sidebar's own header already has a "✕" that hides the sidebar.

## Goals / Non-Goals

**Goals:**
- One exit path for the button and the shortcut.
- No loss of native behaviour that users rely on: moving, maximizing and resizing.

**Non-Goals:**
- Recreating the window manager menu or minimize/maximize buttons (user decision: close only).

## Decisions

### D1. A dedicated title-bar row, not drag regions in existing headers

A new `TitleBar.svelte`, about 32 px high, sits above `app-shell`. It holds the drag area (`data-tauri-drag-region`), the "Vault · Page" title and the close button.

- **Alternative:** turn the existing sidebar and editor headers into drag regions and add the close button to the editor header. This saves 32 px, but puts the app close button next to the right sidebar's own "✕" (hide panel), which is easy to confuse, and drag regions would be scattered between controls.

### D2. Close button reuses the quit flow

The button sets `app.showExitPrompt = true`, the same state `Ctrl+Q` sets, so the confirmation, mutual exclusivity with other modals and `quitApp()` are shared. No new Rust command.

### D3. Dragging and maximizing through Tauri's drag region

`data-tauri-drag-region` on the empty title area handles dragging and double-click maximize. The attribute applies only to the element itself, so the title text gets it too, while buttons do not. Tauri docs (window customization) state that the attribute is not inherited by children, and that buttons and inputs keep working.

### D4. Resize handles call `startResizeDragging`

Eight invisible handles (4 edges at 6 px, 4 corners at 12 px) sit on top of the window border. They set CSS resize cursors and call `getCurrentWindow().startResizeDragging(direction)` on `mousedown`. They are hidden while `isMaximized()` is true, which is updated on the `resized` event. Task 1.3 checks first whether the platform already resizes undecorated windows natively on GNOME and KDE, X11 and Wayland. If it does, the handles must not cause double resizing.

**Task 1.3 findings (source review, 2026-10-30; live check still pending in 3.2).** The Tauri runtime already resizes undecorated windows itself, so the eight DOM handles are not added for now:

- **Linux (GTK, X11 and Wayland alike).** `tauri-runtime-wry` 2.12.3 (`src/undecorated_resizing.rs`, module `gtk`) attaches a handler to every window-content webview. For a toplevel that is undecorated, resizable and not maximized, it hit-tests a 5 px inset (multiplied by the scale factor) on all four edges and corners, sets the matching GDK resize cursor on motion, and on button-1 press calls `gtk::Window::begin_resize_drag(edge, …)` and stops propagation, so the page never sees that press. `begin_resize_drag` is backend-neutral GDK: on X11 it uses `_NET_WM_MOVERESIZE`, on Wayland `xdg_toplevel.resize`, which Mutter (GNOME) and KWin (KDE) both implement. Touch is handled the same way.
- **Maximized.** `resizable_window()` returns nothing while `is_maximized()`, so edges are inactive when maximized, as the spec requires.
- **Windows.** The same module (`windows`) subclasses the HWND and answers `WM_NCHITTEST` for undecorated resizable windows.
- **macOS.** `tao` 0.37.2 creates undecorated windows with `NSWindowStyleMask::Borderless | Resizable`, which AppKit resizes from its edges.
- **Double resizing.** DOM handles inside the 5 px inset would never get the press (GTK stops it first); handles wider than the inset would only add a second, inconsistent hit zone. Adding them would not help.
- **Known caveats from issue trackers.** A tao issue titled "with_decorations(false) doesn't work on wayland kde" (about removing the frame, not about resizing; issue number not confirmed) and the Wayland client-decoration rework in tao PR 979. A web search found no report of broken edge resizing of undecorated windows with Tauri 2.12.
- **Decision.** Rely on the runtime. If the manual pass (3.2) finds a compositor where edges do not resize, add the handles described above, sized to start outside the runtime's 5 px inset, or use the `CERBO_NATIVE_DECORATIONS=1` fallback from Risks.

### D6. Outline drawn by the page, square corners

Without a native frame the window edge blends into windows behind it. `body` draws a 1px outline (`--window-border`, slate in light and dark themes). `TitleBar.svelte` sets `window-maximized` on `<html>` from `isMaximized() || isFullscreen()` on every resize, which removes it. Both queries are in `core:window:default`.

- **Alternative rejected:** rounded corners through a transparent window. The undecorated GTK window keeps a thin square edge of its own, so the rounding drew a second curved line inside it; transparency also shows black corners without a compositor.

### D5. Minimal permissions

The capability adds `core:window:allow-start-dragging`, `core:window:allow-start-resize-dragging` and `core:window:allow-toggle-maximize` (for the drag-region double-click on platforms where the internal path is not enough). It does not add `allow-close`: exit goes through `app_exit`.

## Risks / Trade-offs

- [Wayland compositors handle client-side drag and resize differently (reports of unresponsive client-drawn controls on KDE/Wayland)] → Manual check on GNOME and KDE under Wayland and X11 (task 3.2). If a compositor misbehaves, the fallback is a `CERBO_NATIVE_DECORATIONS=1` environment override that restores the frame; it is added only if the check fails.
- [The window manager's own close (`Alt+F4`) bypasses the confirmation] → Unchanged behaviour, out of scope.
- [macOS loses native window snapping without a title bar] → Accepted. Cerbo's primary platform is Linux.

## Sources

- Tauri window customization (decorations, `data-tauri-drag-region`, permissions): https://v2.tauri.app/learn/window-customization/
- Tauri core permissions list: https://v2.tauri.app/reference/acl/core-permissions/
- `tauri-runtime-wry` 2.12.3, `src/undecorated_resizing.rs` and its call in `src/lib.rs` (`attach_resize_handler` for `WebviewKind::WindowContent`); `tao` 0.37.2, `src/platform_impl/macos/window.rs` (style masks)
- tao PR 979 (Wayland client-side decorations rework): https://github.com/tauri-apps/tao/pull/979
