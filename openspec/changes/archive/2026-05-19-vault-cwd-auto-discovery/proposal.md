## Why

When working inside a vault directory, users must always specify `--vault <id>` even though the vault is obvious from the current working directory. This is unnecessary friction — every git-like tool (git, cargo, etc.) auto-detects the repository from CWD.

## What Changes

- All CLI commands that require a vault context SHALL implicitly use the CWD-discovered vault when `--vault` is not provided.
- The vault walk-up helper (already specified in `vault-management`) is used to discover the vault root from `$CWD`.
- If CWD is not inside any vault and `--vault` is also absent, the command SHALL exit with the existing error: `not a cerbo vault (or any parent up to mount point)`.
- Commands that explicitly receive `--vault <id>` continue to use that vault unchanged.

## Capabilities

### New Capabilities
- `vault-cwd-auto-discovery`: When no `--vault` flag is supplied, cerbo resolves the active vault by walking up from CWD to find `.cerbo/`, then uses that vault as the implicit current vault for the command.

### Modified Capabilities
- `vault-management`: Add requirement that CLI commands use the walk-up helper as the default vault resolver (the helper already exists; the new requirement is that it is wired into the CLI dispatch layer).

## Impact

- **CLI dispatch layer** (`cli/src/main.rs`): vault resolution logic added before command handlers run.
- **All subcommands** that currently require `--vault`: `page`, `index`, `symlink`, `object`, and any others operating on vault content.
- **No API changes**: the walk-up helper already exists in `core`; this only wires it to the CLI.
- **No breaking changes**: `--vault` continues to work exactly as before; this only fills in the default.
