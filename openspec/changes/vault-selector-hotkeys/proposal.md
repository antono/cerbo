## Why

Users need a faster, more discoverable way to switch vaults without relying on the old vault switcher UI. This change also surfaces keyboard shortcuts in the main chrome so hotkeys are easier to learn and use.

## What Changes

- Add a vault selector that opens with `Ctrl+Shift+O` and presents vaults in a selector UI similar to page search.
- Remove the old inline vault switcher from the sidebar header.
- Add a help icon near the theme toggle that opens keyboard shortcuts help.
- Update shortcut help content to include the vault selector hotkey.
- Preserve existing vault open behavior when a vault is selected.

## Capabilities

### New Capabilities
- `vault-selector`: keyboard-triggered vault picker UI for switching vaults and adding new ones.
- `hotkeys-help`: in-app shortcuts help surfaced from the shell chrome.

### Modified Capabilities
- `keyboard-shortcuts`: shortcut inventory and help content now includes vault selection and shell-level help access.

## Impact

- `src/routes/+layout.svelte`: global hotkey handling, header actions, dialog orchestration.
- `src/lib/VaultSwitcher.svelte`: replacement or removal of the old vault switcher UI.
- `src/lib/KeyboardHelp.svelte`: updated shortcut list and entry point.
- `src/lib/ThemeToggle.svelte`: header area rework to add a help icon nearby.
- `src/lib/stores.svelte.ts`: UI state for vault selector/help dialogs and existing vault open flow.
