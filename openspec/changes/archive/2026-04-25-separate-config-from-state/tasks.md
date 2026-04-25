## 1. Add TOML Dependency

- [x] 1.1 Add `toml` crate to `core/Cargo.toml`

## 2. Create Config Module

- [x] 2.1 Create `core/src/config.rs` with TOML serialization
- [x] 2.2 Implement `Config` struct with `vaults` field
- [x] 2.3 Implement `load_config()` function
- [x] 2.4 Implement `save_config()` function (atomic write)

## 3. Create UI Settings Module

- [x] 3.1 Create `core/src/ui_settings.rs`
- [x] 3.2 Implement `UiSettings` struct (theme, font_size, sidebar_width)
- [x] 3.3 Implement `load_ui_settings()` function
- [x] 3.4 Implement `save_ui_settings()` function

## 4. Create State Module

- [x] 4.1 Create `core/src/state.rs`
- [x] 4.2 Implement `State` struct (active_vault_id, vault_states)
- [x] 4.3 Implement `load_state()` function
- [x] 4.4 Implement `save_state()` function

## 5. Create Migration Module

- [x] 5.1 Create `core/src/migration.rs`
- [x] 5.2 Implement `migrate_if_needed()` function
- [x] 5.3 Add TODO comment: Remove after v0.3.0
- [x] 5.4 Test migration with sample JSON config

## 6. Update Vault Module

- [x] 6.1 Remove `last_open_page` from Vault struct (moved to state)
- [x] 6.2 Update `load_vaults()` to use new config module
- [x] 6.3 Update save functions to use TOML

## 7. Integrate with Startup

- [x] 7.1 Call `migrate_if_needed()` before loading config
- [x] 7.2 Wire up config, ui_settings, state loading in CLI startup
- [x] 7.3 Wire up config, ui_settings, state loading in desktop startup

## 8. Test and Verify

- [x] 8.1 Test fresh start creates vaults.toml, ui.toml, state.toml
- [x] 8.2 Test migration from old vaults.json works
- [x] 8.3 Test active vault persists to state.toml
- [x] 8.4 Test last_open_page persists to state.toml
- [x] 8.5 Test UI settings persist to ui.toml
- [x] 8.6 Verify files in correct XDG directories
