## Context

`cerbo` currently has two vault resolution paths that are inconsistent across commands:

- `index` and `symlink` already call `VaultContext::from_cwd()` / `find_vault_root()` when `--vault` is absent — they work correctly inside a vault.
- `page` subcommands (create/read/write/delete/list), `resolve`, `import`, and `info` all use the global XDG `CerboContext` (`~/.config/cerbo/`) regardless of CWD — they are vault-unaware.

The `CerboContext` used for page operations must point to the vault's `.cerbo/` dir, not the global config dir. A vault is identified by its registered ID; the walk-up helper returns a path, which must be matched against the registered vaults list to get an ID.

## Goals / Non-Goals

**Goals:**
- CWD-inside-a-vault implicitly sets the effective vault for **all** CLI commands.
- Explicit `--vault <id>` continues to work and takes precedence.
- Consistent behavior: same resolution logic across every subcommand.
- Zero breaking changes: flags, outputs, and error codes remain identical.

**Non-Goals:**
- Auto-registering discovered vaults (user must still run `cerbo vault add`).
- Changing the `VaultContext` API used by `index` / `symlink` (already correct).
- Affecting `cerbo vault` management subcommands (they operate on the registry, not on vault content).
- Affecting `cerbo init` (it always operates on CWD by design).

## Decisions

### Decision 1 — Resolve effective vault once at `main()` startup

Walk up from CWD once at the beginning of `fn main()`, before match-dispatching commands. Store the result as `Option<PathBuf>` (vault root). Pass it into helper that returns `Option<String>` (vault ID) by matching the discovered path against the registered vaults list.

```
Priority chain (highest to lowest):
  1. Explicit --vault <id> flag on the command
  2. CWD-discovered vault (walk-up finds .cerbo/, matched to registered vault ID)
  3. Active vault from state (existing behaviour)
  4. Single registered vault fallback (existing behaviour)
  5. Error: "not a cerbo vault"
```

**Alternative considered:** resolve per-command. Rejected — duplicates logic, leaves gaps when new commands are added.

### Decision 2 — CWD discovery returns path; path is matched to registered vault ID

`find_vault_root(&cwd)` already exists in `cerbo_core::vault`. A new thin helper `vault_id_from_path(ctx, root) -> Option<String>` scans the registered vaults list and returns the ID whose `path` equals the discovered root.

If the vault at CWD is not registered, CWD discovery yields `None` for the ID (vault exists on disk but isn't known to the registry). The existing active-vault / single-vault fallback then applies. Commands that can operate path-only (`index`, `symlink`) are unaffected.

**Alternative considered:** Accept a filesystem path as `--vault` value. Rejected — would be a breaking change to the flag semantics.

### Decision 3 — Build effective `CerboContext` from resolved vault ID

A helper `effective_ctx(global_ctx, vault_id: Option<&str>) -> Result<CerboContext, String>` replaces the ad-hoc per-command vault lookup in `page list`. It looks up the vault by ID and returns a `CerboContext` whose `config_dir` points to `<vault-path>/.cerbo/`.

All page subcommands (and `resolve`, `import`, `import-ontology`) switch from using the global `ctx` to using `effective_ctx(ctx, resolved_vault_id)`.

### Decision 4 — `page list` keeps `--vault` as optional vault ID override

`PageCommands::List { vault: Option<String> }` already exists. The explicit `--vault` value is used as-is (priority 1). If absent, the resolved vault from CWD/state is used (priorities 2–4). No flag changes.

Other page commands (`create`, `read`, `write`, `delete`) do not currently have `--vault`; they gain vault awareness implicitly through the resolved context without any flag changes.

## Risks / Trade-offs

- **Unregistered vault at CWD** → CWD discovery resolves to `None` for vault ID, falls through to active-vault state. User sees no error, but commands operate on the wrong vault. Mitigation: emit a warning on stderr when a `.cerbo/` is found at CWD but not in the registry (`warning: vault at <path> is not registered; run 'cerbo vault add'`).

- **Registered path mismatch** (symlinks, trailing slashes) → `vault_id_from_path` must canonicalize both paths before comparing. Use `std::fs::canonicalize`.

- **Performance** → `find_vault_root` + `vault_list` read on every invocation. Both are small file reads; negligible.

## Migration Plan

No migration needed. The change is additive: commands that previously ignored CWD will now respect it. Users relying on the active-vault state are unaffected as long as the active vault matches their CWD vault (it becomes the CWD discovery result).

## Open Questions

None — design is fully constrained by the existing code structure.
