## Why

Manually typing or pasting filesystem paths when adding a vault is error-prone and a poor user experience. Users expect a standard native directory picker to select where their wiki data is stored, including the ability to create new folders during selection.

## What Changes

- Integration of `tauri-plugin-dialog` to provide native OS folder selection.
- Update the "Add Vault" UI to trigger the native folder picker.
- Support for selecting an existing directory or creating a new directory as a vault root.

## Capabilities

### New Capabilities
- `native-vault-selection`: Capability to pick a local folder using native OS dialogs for vault registration.

### Modified Capabilities
- `vault-management`: Update registration flow to accept paths from the native dialog.

## Impact

- **Dependencies**: Add `tauri-plugin-dialog` to Rust (`Cargo.toml`) and Frontend (`package.json`).
- **Rust**: Register `tauri-plugin-dialog` in `lib.rs`.
- **Frontend**: Update `VaultSwitcher.svelte` to use the dialog plugin.
- **Config**: Update `capabilities/default.json` to allow dialog permissions.
