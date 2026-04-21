## 1. Dependencies and Plugin Setup

- [ ] 1.1 Add `tauri-plugin-dialog` to Rust dependencies in `src-tauri/Cargo.toml`
- [ ] 1.2 Add `@tauri-apps/plugin-dialog` to frontend dependencies in `package.json`
- [ ] 1.3 Register `tauri-plugin-dialog` in `src-tauri/src/lib.rs`
- [ ] 1.4 Add `dialog:allow-open` permission to `src-tauri/capabilities/default.json`

## 2. Frontend Implementation

- [ ] 2.1 Import `open` from `@tauri-apps/plugin-dialog` and `homeDir` from `@tauri-apps/api/path` in `src/lib/VaultSwitcher.svelte`
- [ ] 2.2 Add a "Browse" button next to the vault path input in `VaultSwitcher.svelte`
- [ ] 2.3 Implement the `selectFolder` handler to open the native dialog and update the input state

## 3. Verification

- [ ] 3.1 Verify that clicking "Browse" opens the native OS folder selection dialog
- [ ] 3.2 Verify that selecting an existing folder populates the path input
- [ ] 3.3 Verify that creating a new folder and selecting it populates the path input
- [ ] 3.4 Verify that adding a vault using the picked path works correctly
