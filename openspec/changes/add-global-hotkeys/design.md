## Context

The application currently has a global hotkey listener in `src/routes/+layout.svelte` that handles `Ctrl+N` (New Page), `Ctrl+P` (Search), and `Ctrl+Q` (Quit). There is no help menu for shortcuts, and common actions like theme switching and adding vaults require multiple clicks.

## Goals / Non-Goals

**Goals:**
- Provide global shortcuts for theme toggling (`Ctrl+T`), help (`F1`), and adding vaults (`Ctrl+O`).
- Centralize shortcut-triggered commands in the global store for reusability.
- Create a user-friendly reference for all application shortcuts.

**Non-Goals:**
- User-customizable hotkeys (remappable keys).
- Conflict detection between shortcuts and OS-level hotkeys beyond what's handled by `isModKey`.

## Decisions

### 1. Centralized "Quick Add Vault" logic
**Decision**: Move the logic for opening the directory picker and adding a vault from `VaultSwitcher.svelte` to `stores.svelte.ts`.
**Rationale**: This logic needs to be triggered from both the switcher UI and the global `Ctrl+O` hotkey. Centralizing it prevents duplication and ensures consistent behavior.

### 2. Global `showHelp` state
**Decision**: Add `showHelp: false` to the `app` reactive state in `stores.svelte.ts`.
**Rationale**: This allows any part of the application (like the layout listener) to toggle the help modal visibility. It will be added to the `closeAllDialogs` helper.

### 3. Native-style Help Modal
**Decision**: Implement `KeyboardHelp.svelte` using the existing modal pattern (background overlay, centered dialog, Svelte transitions).
**Rationale**: Consistency with `GlobalSearch` and `ExitConfirmation` components.

### 4. Direct Theme Toggling in Hotkey Listener
**Decision**: Use `mode-watcher`'s `setMode` directly in the `handleKeydown` function.
**Rationale**: `setMode` is already a simple utility provided by the library. No need for extra indirection unless the toggle logic becomes more complex.

## Risks / Trade-offs

- **[Risk]** Hotkey collisions with browser or OS (e.g., `Ctrl+T` is "New Tab" in browsers).
- **[Mitigation]** Since this is a Tauri app, most browser-level shortcuts are intercepted by the application window. We will call `e.preventDefault()` to ensure our actions take precedence.

- **[Risk]** Modal stacking/interference.
- **[Mitigation]** Use the existing `closeAllDialogs()` pattern before opening a new modal to ensure only one transient UI element is active at a time.
