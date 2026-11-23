# Tasks

## 1. Tauri
- [x] 1.1 `vault_import_plan` command (spawn_blocking, returns `ImportReport`)
- [x] 1.2 `vault_import_run` command with `{existing|new}` target and `force`
- [ ] 1.3 Register handlers; Rust tests for target-inside-source and repeat refusal

## 2. Frontend
- [x] 2.1 `vault.import` action (palette, vault selector, empty-state toolbar)
- [x] 2.2 `ImportVaultDialog.svelte`: steps source (Obsidian first) → folder → target → preview
- [x] 2.3 Format mismatch hint using `detect`
- [x] 2.4 Store: `importVault()` → `loadVaults()` → `openVault()`
- [x] 2.5 Keyboard: Enter next, Esc close (inert while running)

## 3. Tests
- [x] 3.1 Vitest for step logic and mismatch hint
- [ ] 3.2 e2e: import `core/tests/fixtures/import/obsidian` into a new vault

## 4. Docs
- [ ] 4.1 Lift "Desktop UI" non-goal in `vault-import` spec; hotkeys help if a shortcut is added
