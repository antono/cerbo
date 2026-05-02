## Why

CLI commands currently output human-readable text mixed with data, making it difficult to parse output programmatically or integrate with other tools. Adding a `--json` flag will enable machine-readable JSON output for all CLI commands, supporting scripting, automation, and tool integration.

## What Changes

- Add `--json` flag to all CLI commands
- When `--json` is specified, output only valid JSON data to stdout
- Suppress all non-JSON output (log messages, status text, etc.) when `--json` is used
- JSON output must be the only content on stdout (no wrapping text, no decorative output)

## Capabilities

### New Capabilities
- `cli-json-output`: JSON output format for all CLI commands with `--json` flag support

### Modified Capabilities
<!-- No existing capabilities require spec-level changes -->

## Impact

- **Code**: All CLI command implementations in `cli` crate (likely `src/cli/` or similar)
- **Output formatting**: Need to identify all output points and add JSON serialization
- **Dependencies**: May need a JSON serialization library (serde_json likely already available in Rust workspace)
- **Testing**: Need to verify JSON output is valid and contains expected data structure for each command
