## Why

The application needs preview mode keyboard shortcuts to allow users to scroll content and navigate the sidebar efficiently without leaving the keyboard. Current `j/k` bindings navigate pages, but users expect them to scroll like standard vim-style navigation. Also, vault shortcuts need to match user mental models: quick vault selection via `Ctrl+O` and adding vaults via `Ctrl+Shift+O`.

## What Changes

- Change `j`/`k` in preview mode to scroll viewport (not navigate pages)
- Add `J` (uppercase) to navigate to next page in sidebar
- Add `K` (uppercase) to navigate to previous page in sidebar
- Swap `Ctrl+O` and `Ctrl+Shift+O` behaviors:
  - `Ctrl+O` → Select vault (was Add Vault)
  - `Ctrl+Shift+O` → Add vault (was Select Vault)

## Capabilities

### New Capabilities
- `preview-scroll-hotkeys`: Preview mode scroll shortcuts (`j`/`k`) and sidebar navigation (`J`/`K`)

### Modified Capabilities
- `keyboard-shortcuts`: Update shortcut definitions to match new behaviors
- `global-hotkeys`: Update vault shortcut registrations (swap Ctrl+O and Ctrl+Shift+O)

## Impact

- `src/lib/components/`: Update preview component to handle scroll vs navigate
- `src/lib/components/Sidebar.svelte`: Add `J`/`K` handling for sidebar navigation
- `src-tauri/`: Swap global hotkey registrations for vault actions
- Help/shortcuts display: Update to reflect new key bindings
