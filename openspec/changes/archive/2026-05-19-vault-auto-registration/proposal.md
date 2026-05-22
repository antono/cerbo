## Why

Currently, a vault discovered from CWD is used implicitly but is never persisted to the registry. This means commands like `cerbo vault list` never show auto-discovered vaults, and the vault must be manually registered with `cerbo vault add` before it becomes a first-class citizen. Auto-registering discovered vaults removes this friction while keeping intentional (`vault add`) registrations separate and authoritative.

## What Changes

- When the CWD walk-up discovers a vault that is not yet in the registry, cerbo SHALL automatically record it in a separate **auto-registered vaults** file (e.g. `vaults.auto.toml`) stored in the XDG config dir.
- `core` exposes a single `vault_list` that merges both files and tags each entry with `is_auto`. All callers (CLI, `resolve_vault_ctx`, `vault_id_from_path`) work with this unified list — no caller-side merging required.
- The existing `vaults.toml` (manually registered vaults) is **not modified** by auto-registration — it remains under full user control.
- `cerbo vault list` shows auto-registered vaults with a visual marker (e.g. `(auto)`) so users know they were not manually added.
- A new `cerbo vault approve <id>` command promotes an auto-registered vault to the manual registry, moving the entry from `vaults.auto.toml` to `vaults.toml`.
- Auto-registered vaults can be removed with `cerbo vault remove --auto <id>` or `cerbo vault remove <id>` (works on both lists).
- The unregistered-vault warning introduced in `vault-cwd-auto-discovery` is **removed** — auto-registration replaces the warning.

## Capabilities

### New Capabilities
- `vault-auto-registration`: When CWD walk-up finds a vault not in the manual registry, automatically record it in a separate auto-vaults store. Define the auto-registration lifecycle (discover → auto-register → optionally promote to manual).

### Modified Capabilities
- `vault-management`: Add requirements for auto-registered vault list, `vault list` display (with `(auto)` marker), new `vault approve` subcommand for promotion, and removal via `vault remove`.

## Non-goals

- Auto-registering vaults found outside the CWD walk-up (e.g. filesystem scans).
- Merging `vaults.auto.toml` with `vaults.toml` at any point automatically.
- Changing vault priority ordering (auto-registered vaults rank the same as manually registered for resolution purposes).
- UI/desktop changes — CLI only.

## Impact

- **`core/src/vault.rs`**: `vault_list` returns a unified list of all vaults (manual + auto-registered), each entry carrying an `is_auto: bool` flag. Separate internal helpers manage the two backing files (`vaults.toml`, `vaults.auto.toml`); callers never need to merge lists themselves.
- **`core/src/config.rs`**: new `AutoVaultsConfig` type and `vaults.auto.toml` path helper.
- **`cli/src/main.rs`**: `resolve_vault_ctx` updated to also search auto-registered vaults; startup discovery calls `auto_vault_add` when a new vault is found; `vault list` updated to show `(auto)` marker; new `VaultCommands::Approve` variant wired to `auto_vault_approve`.
- **No breaking changes** to existing `vaults.toml` format or vault IDs.
