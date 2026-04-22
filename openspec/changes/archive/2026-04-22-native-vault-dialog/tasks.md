## 1. Dependencies and Plugin Setup

- [x] 1.1 Add `tauri-plugin-dialog` to Rust dependencies in `src-tauri/Cargo.toml`
- [x] 1.2 Add `@tauri-apps/plugin-dialog` to frontend dependencies in `package.json`
- [x] 1.3 Register `tauri-plugin-dialog` in `src-tauri/src/lib.rs`
- [x] 1.4 Add `dialog:allow-open` permission to `src-tauri/capabilities/default.json`

## 2. Frontend Implementation

- [x] 2.1 Import `open` from `@tauri-apps/plugin-dialog` and `homeDir` from `@tauri-apps/api/path` in `src/lib/VaultSwitcher.svelte`
- [x] 2.2 Add a "Browse" button next to the vault path input in `VaultSwitcher.svelte`
- [x] 2.3 Implement the `selectFolder` handler to open the native dialog and update the input state

## 3. Verification

- [x] 3.1 Verify that clicking "Browse" opens the native OS folder selection dialog
- [x] 3.2 Verify that selecting an existing folder populates the path input
- [x] 3.3 Verify that creating a new folder and selecting it populates the path input
- [x] 3.4 Verify that adding a vault using the picked path works correctly
