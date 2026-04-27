# Vault Selector

## Purpose
Provide a modal vault selector for switching, adding, and managing vault access from the keyboard.

## Requirements

### Requirement: Keyboard-triggered vault selector
The application MUST open a vault selector when the user presses `Ctrl+Shift+O` (`Cmd+Shift+O` on macOS).

### Requirement: Vault selector behavior
The vault selector MUST present available vaults in a modal selector UI consistent with the existing page selector interaction model.

### Requirement: Vault switching
Selecting a vault in the selector MUST open that vault using the existing vault open flow.

### Requirement: Add vault from selector
The vault selector MUST provide a way to add a new vault.

### Requirement: Replace old switcher
The old inline vault switcher MUST no longer be shown in the sidebar header.
