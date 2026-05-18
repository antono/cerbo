## 1. Core helper in cerbo_core

- [x] 1.1 Add `vault_id_from_path(ctx: &CerboContext, root: &Path) -> Option<String>` to `core/src/vault.rs` — canonicalize both paths before comparing
- [x] 1.2 Export `vault_id_from_path` from `core/src/lib.rs`
- [x] 1.3 Add unit tests for `vault_id_from_path`: match found, no match, symlink path resolves correctly

## 2. CLI startup vault resolution

- [x] 2.1 In `cli/src/main.rs` `fn main()`, after building `ctx`, call `find_vault_root(&cwd)` to get `Option<PathBuf>`
- [x] 2.2 Call `vault_id_from_path(&ctx, &root)` to get `Option<String>` (cwd vault ID)
- [x] 2.3 If walk-up found `.cerbo/` but `vault_id_from_path` returns `None`, emit `warning: vault at <path> is not registered; run 'cerbo vault add'` to stderr
- [x] 2.4 Extract `resolve_effective_vault_id(ctx, cwd_vault_id, explicit_vault_id) -> Option<String>` helper applying the 5-level priority chain: explicit flag → CWD discovery → active state → single-vault fallback → None
- [x] 2.5 Add `effective_ctx(global_ctx: &CerboContext, vault_id: Option<&str>) -> Result<CerboContext, String>` helper that looks up vault by ID and returns a context pointing at `<vault-path>/.cerbo/`

## 3. Wire effective context into commands

- [x] 3.1 `PageCommands::List` — replace per-command vault lookup with `resolve_effective_vault_id` + `effective_ctx`; remove duplicated vault-lookup block
- [x] 3.2 `PageCommands::Create` — switch from global `ctx` to `effective_ctx`
- [x] 3.3 `PageCommands::Read` — switch from global `ctx` to `effective_ctx`
- [x] 3.4 `PageCommands::Write` — switch from global `ctx` to `effective_ctx`
- [x] 3.5 `PageCommands::Delete` — switch from global `ctx` to `effective_ctx`
- [x] 3.6 `Commands::Resolve` — switch from global `ctx` to `effective_ctx`
- [x] 3.7 `Commands::Import` — switch from global `ctx` to `effective_ctx`
- [x] 3.8 `Commands::ImportOntology` — switch from global `ctx` to `effective_ctx`
- [x] 3.9 `Commands::Info` — include CWD-discovered vault in output (mark as `(current)` when matched)

## 4. Consolidate existing CWD resolution in index and symlink

- [x] 4.1 `Commands::Index` — replace inline `VaultContext::from_cwd()` with the shared `resolve_effective_vault_id` path so behaviour is consistent (keep `VaultContext` construction, just align the vault source)
- [x] 4.2 `Commands::Symlink` — replace inline `find_vault_root` block with shared resolution; keep existing error handling

## 5. Update man page

- [x] 5.1 Update `cli/man/cerbo.1` to document that all commands auto-detect the vault from CWD when `--vault` is not supplied

## 6. Tests

- [x] 6.1 Integration test: run `cerbo page list` from inside a registered vault dir — verify pages from that vault are returned
- [x] 6.2 Integration test: run `cerbo page list --vault <other-id>` from inside a different vault — verify explicit flag wins
- [x] 6.3 Integration test: run `cerbo page list` from a directory with `.cerbo/` that is not registered — verify warning on stderr and graceful fallback
- [x] 6.4 Integration test: run `cerbo page list` from outside any vault with no active vault set — verify non-zero exit with clear error
