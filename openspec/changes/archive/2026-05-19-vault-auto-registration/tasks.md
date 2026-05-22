## 1. Core data model

- [x] 1.1 Add `is_auto: bool` field to `Vault` struct in `core/src/vault.rs` with `#[serde(default)]` so existing `vaults.toml` files deserialise cleanly
- [x] 1.2 Add `auto_config_path(ctx) -> Result<PathBuf, String>` helper in `core/src/config.rs` returning `config_dir/vaults.auto.toml`
- [x] 1.3 Add `load_auto_vaults(ctx) -> Result<VaultsFile, String>` and `save_auto_vaults(ctx, &VaultsFile) -> Result<(), String>` in `core/src/vault.rs` — same TOML shape as `load_vaults`

## 2. Unified vault_list

- [x] 2.1 Update `vault_list(ctx)` to merge manual + auto entries: load both files, tag auto entries with `is_auto = true`, suppress auto entries whose path duplicates a manual entry
- [x] 2.2 Update `vault_id_from_path` to use the unified list (no change if it already calls `vault_list`)
- [x] 2.3 Update `get_vault_path` to search unified list
- [x] 2.4 Add unit tests: mixed list, dedup when same path in both files, empty auto file

## 3. Auto-registration on CWD discovery

- [x] 3.1 Add `auto_vault_register(ctx, root: &Path) -> Result<(), String>` in `core/src/vault.rs` — derives name from directory name, generates UUID, skips if path already in either registry, appends to `vaults.auto.toml`
- [x] 3.2 In `cli/src/main.rs` startup block, replace the unregistered-vault warning with a call to `auto_vault_register(&ctx, root)` when `cwd_vault_id` is `None` and `cwd_vault_root` is `Some`
- [x] 3.3 Re-resolve `cwd_vault_id` after auto-registration so the newly registered vault is used in the same invocation
- [x] 3.4 Add unit tests for `auto_vault_register`: new vault added, idempotent on second call, skipped when path in manual registry

## 4. vault approve command

- [x] 4.1 Add `auto_vault_approve(ctx, id: &str) -> Result<Vault, String>` in `core/src/vault.rs` — finds entry in `vaults.auto.toml`, appends to `vaults.toml` with `is_auto = false`, removes from `vaults.auto.toml`, errors if not found or path already in manual registry
- [x] 4.2 Add `VaultCommands::Approve { id: String, json: bool }` variant to the CLI enum in `cli/src/main.rs`
- [x] 4.3 Wire `vault approve` in the match arm: call `auto_vault_approve`, print confirmation or JSON
- [x] 4.4 Add `vault remove` support for auto entries: update `vault_remove` in core to search both files

## 5. vault list display

- [x] 5.1 Update `print_vaults_template` in `cli/src/main.rs` to append ` (auto)` when `v.is_auto`
- [x] 5.2 Update `VaultJson` struct to include `is_auto: bool` field for JSON output
- [x] 5.3 Update `VaultInfoJson` struct to include `is_auto: bool` for `info --json` output

## 6. Man page

- [x] 6.1 Document `cerbo vault approve <ID> [--json]` subcommand in `cli/man/cerbo.1`
- [x] 6.2 Update `cerbo vault list` description to mention `(auto)` marker and `is_auto` JSON field

## 7. Tests

- [x] 7.1 Integration test: run cerbo from inside an unregistered vault, verify it appears in `vault list --json` with `is_auto: true` on second invocation
- [x] 7.2 Integration test: `cerbo vault approve <id>` promotes vault, subsequent `vault list --json` shows `is_auto: false`
- [x] 7.3 Integration test: `cerbo vault remove <id>` works on an auto-registered vault
- [x] 7.4 Integration test: auto-registration is idempotent — running cerbo twice from the same vault creates only one entry
