## 1. Add TOML Dependency

- [ ] 1.1 Add `toml` crate to `core/Cargo.toml`

## 2. Create Config Module

- [ ] 2.1 Create `core/src/config.rs` with TOML serialization
- [ ] 2.2 Implement `Config` struct with `vaults` field
- [ ] 2.3 Implement `load_config()` function
- [ ] 2.4 Implement `save_config()` function (atomic write)

## 3. Create UI Settings Module

- [ ] 3.1 Create `core/src/ui_settings.rs`
- [ ] 3.2 Implement `UiSettings` struct (theme, font_size, sidebar_width)
- [ ] 3.3 Implement `load_ui_settings()` function
- [ ] 3.4 Implement `save_ui_settings()` function

## 4. Create State Module

- [ ] 4.1 Create `core/src/state.rs`
- [ ] 4.2 Implement `State` struct (active_vault_id, vault_states)
- [ ] 4.3 Implement `load_state()` function
- [ ] 4.4 Implement `save_state()` function

## 5. Create Migration Module

- [ ] 5.1 Create `core/src/migration.rs`
- [ ] 5.2 Implement `migrate_if_needed()` function
- [ ] 5.3 Add TODO comment: Remove after v0.3.0
- [ ] 5.4 Test migration with sample JSON config

## 6. Update Vault Module

- [ ] 6.1 Remove `last_open_page` from Vault struct (moved to state)
- [ ] 6.2 Update `load_vaults()` to use new config module
- [ ] 6.3 Update save functions to use TOML

## 7. Integrate with Startup

- [ ] 7.1 Call `migrate_if_needed()` before loading config
- [ ] 7.2 Wire up config, ui_settings, state loading in CLI startup
- [ ] 7.3 Wire up config, ui_settings, state loading in desktop startup

## 8. Test and Verify

- [ ] 8.1 Test fresh start creates config.toml, ui.toml, state.toml
- [ ] 8.2 Test migration from old vaults.json works
- [ ] 8.3 Test active vault persists to state.toml
- [ ] 8.4 Test last_open_page persists to state.toml
- [ ] 8.5 Test UI settings persist to ui.toml
- [ ] 8.6 Verify files in correct XDG directories