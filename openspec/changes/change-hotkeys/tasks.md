## 1. Swap Vault Shortcuts

- [ ] 1.1 Update Tauri global hotkey registration: change Ctrl+O to open vault selector
- [ ] 1.2 Update Tauri global hotkey registration: change Ctrl+Shift+O to trigger add vault
- [ ] 1.3 Update keyboard-shortcuts spec references to reflect swapped shortcuts

## 2. Preview Mode Scroll Hotkeys

- [ ] 2.1 Add `j` key handler in preview mode to scroll viewport down by 100px using scrollBy API
- [ ] 2.2 Add `k` key handler in preview mode to scroll viewport up by 100px using scrollBy API
- [ ] 2.3 Add input focus check to suppress `j`/`k` scroll when an input or textarea is focused
- [ ] 2.4 Remove `j`/`k` page navigation logic from preview mode (now handled by `J`/`K`)

## 3. Sidebar Navigation with J/K

- [ ] 3.1 Add `J` (uppercase) key handler to navigate to next page in sidebar list
- [ ] 3.2 Add `K` (uppercase) key handler to navigate to previous page in sidebar list
- [ ] 3.3 Add input focus check to suppress `J`/`K` when an input or textarea is focused
- [ ] 3.4 Update sidebar focus state to follow `J`/`K` navigation

## 4. Update Help and Documentation

- [ ] 4.1 Update Shortcuts Help modal to show `j`/`k` as "Scroll down"/"Scroll up"
- [ ] 4.2 Update Shortcuts Help modal to show `J`/`K` as "Next page"/"Previous page"
- [ ] 4.3 Update Shortcuts Help modal to reflect swapped vault shortcuts (Ctrl+O = Select, Ctrl+Shift+O = Add)
- [ ] 4.4 Verify cross-platform key mapping (Ctrl→Cmd on Mac) for all changed shortcuts
