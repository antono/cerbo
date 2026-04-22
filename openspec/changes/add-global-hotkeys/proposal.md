## Why

Improve application accessibility and power-user efficiency by providing standard keyboard shortcuts for frequent core actions: theme switching, accessing help, and adding new vaults.

## What Changes

- Add `Ctrl+T` (Cmd+T on macOS) shortcut to toggle between Light and Dark themes.
- Add `F1` shortcut to open a global help modal displaying all registered keyboard shortcuts.
- Add `Ctrl+O` (Cmd+O on macOS) shortcut to trigger the "Add Vault" command (opens native directory picker).
- Implement a new "Keyboard Shortcuts Help" modal.
- Update relevant specifications to include these new interaction requirements.

## Capabilities

### New Capabilities
- `shortcuts-help`: A centralized help interface (modal/dialog) that provides users with a quick reference for all active application-level keyboard shortcuts, triggered by a dedicated hotkey (F1).

### Modified Capabilities
- `keyboard-shortcuts`: Add explicit requirements for Theme Toggle (Ctrl+T), Shortcuts Help (F1), and Add Vault (Ctrl+O) global hotkeys.
- `theme-management`: Include the keyboard shortcut as a requirement for toggling the application theme state.
- `vault-management`: Include the keyboard shortcut as a requirement for triggering the vault registration workflow.

## Impact

- `src/lib/hotkeys.ts`: Update global keyboard listener to handle new shortcuts.
- `src/lib/stores.svelte.ts`: Ensure theme toggle and vault addition logic are accessible to the hotkey system.
- `src/lib/KeyboardHelp.svelte` (New): Create a new component for the shortcuts help modal.
- `openspec/specs/`: Update corresponding specification files and create delta specs for the current change.
