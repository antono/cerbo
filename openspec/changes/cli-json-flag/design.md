## Context

The Cerbo CLI currently outputs human-readable text using `println!` macros throughout `cli/src/main.rs`. Commands like `vault list`, `page read`, and `info` output data mixed with descriptive text (e.g., "Added vault: {:#?}", "Deleted page"). This makes it difficult to parse output programmatically or integrate with automation tools, scripts, or other CLI tools.

The CLI is built using the `clap` crate for argument parsing, with subcommands organized as `Commands`, `VaultCommands`, `PageCommands`, `AttachmentCommands`, and `IndexCommands` enums. The actual operations are delegated to `cerbo_core` functions.

## Goals / Non-Goals

**Goals:**
- Add `--json` flag to all CLI commands and subcommands
- Output only valid JSON when `--json` is specified
- Maintain backward compatibility (non-JSON output unchanged)
- Use `serde_json` for serialization (already available via `cerbo_core`)

**Non-Goals:**
- Changing the human-readable output format
- Adding JSON output to the Tauri/desktop app
- Supporting other output formats (XML, YAML, etc.)
- Modifying `cerbo_core` function signatures

## Decisions

### Decision 1: Add `--json` flag globally via Clap's `ArgAction::SetTrue`

**Rationale**: Adding `--json` as a global flag using `#[command(global_setting = clap::AppSettings::...)]` or by adding a `#[arg(long)] json: bool` field to the main `Cli` struct. This ensures all subcommands inherit the flag without modifying each enum individually.

**Alternatives considered:**
- Adding `--json` to each subcommand individually → More code duplication, harder to maintain
- Using a wrapper function for JSON output → Over-engineering for this use case

**Decision**: Add `#[arg(long)] pub json: bool` to the main `Cli` struct so it's available to all subcommands.

### Decision 2: Create helper functions for JSON output

**Rationale**: To avoid repeating JSON serialization logic, create helper functions:
- `print_json<T: Serialize>(value: &T)` - serializes and prints JSON
- `print_json_success(message: &str)` - prints `{"success": true, "message": "..."}`
- `print_json_error(message: &str)` - prints `{"error": true, "message": "..."}` 

**Alternatives considered:**
- Inline `serde_json::to_string()` calls → More code duplication
- Creating a trait for JSON output → Over-engineering

**Decision**: Create simple helper functions in `cli/src/main.rs` or a new `cli/src/output.rs` module.

### Decision 3: Suppress non-JSON output when `--json` is used

**Rationale**: When `--json` is specified, all `println!` statements that output status text (e.g., "Added vault", "Deleted page") should be suppressed. Only the JSON output should be printed to stdout.

**Implementation approach**:
- Check `cli.json` flag at the start of each match arm
- If `json` is true, call JSON helper and return early
- Otherwise, proceed with current `println!` output

**Alternatives considered:**
- Using `env_logger` or similar to suppress logs → Too heavy for this use case
- Redirecting stdout → Unnecessary complexity

### Decision 4: JSON structure for different command types

**Rationale**: Define consistent JSON output structures:
- **List commands** (vault list, page list, attachment list, backlinks): Return JSON array of objects
- **Create/Add commands** (vault add, page create, attachment add): Return object with relevant fields (e.g., `{"slug": "..."}` for page create)
- **Update commands** (vault active, page write, page rename): Return `{"success": true, "message": "..."}`
- **Delete commands** (vault remove, page delete, attachment delete): Return `{"success": true, "message": "..."}`
- **Read commands** (page read): Return `{"content": "..."}`
- **Info command**: Return object with config_dir, cache_dir, vaults array
- **Watch command**: Each event outputs `{"event": "...", "vault_id": "...", "message": "..."}`

**Decision**: Follow the structures outlined in the specs artifact.

### Decision 5: Error handling with `--json`

**Rationale**: When errors occur with `--json` flag:
- Catch errors using `?` operator with custom error handling
- Output `{"error": true, "message": "...", "code": ...}` to stdout
- Exit with non-zero code

**Implementation approach**: Wrap `main()` logic in a `match` that catches `Err(e)` and prints JSON error if `--json` is set.

**Alternatives considered:**
- Printing errors to stderr (standard practice) → Doesn't work well when scripts expect only JSON on stdout
- Using different JSON structure for errors → Keep it simple with `error: true` field

## Risks / Trade-offs

**[Risk] Breaking existing scripts that parse human-readable output** → Not applicable since we're not changing non-JSON output

**[Risk] Adding `--json` flag increases complexity of match arms in main.rs** → Mitigation: Use helper functions and early returns to keep code clean

**[Risk] JSON output may be large for commands like `page list` with many pages** → Mitigation: This is expected behavior; users can pipe to `jq` for filtering

**[Risk] Inconsistent JSON structure across commands** → Mitigation: Define structures in specs artifact and follow them strictly

**[Trade-off] Adding dependency on `serde_json` in CLI crate** → `cerbo_core` likely already uses it; if not, it's a standard crate with minimal overhead

**[Trade-off] More code in `main.rs` to handle JSON flag** → Acceptable since it's a cross-cutting feature; consider refactoring to separate module if code grows too large

## Open Questions

1. **Should `--json` flag also suppress error output to stderr?** Currently, Rust's `?` operator and `println!` for errors may still print to stderr. Need to ensure clean JSON-only output.

2. **Should we add `--json` to help subcommand?** The `help` command is auto-generated by `clap`; may not need JSON output.

3. **What should the JSON structure be for `vault list`?** Current code uses `{:#?}` (debug format). Need to define proper struct with id, name, path, active fields.
