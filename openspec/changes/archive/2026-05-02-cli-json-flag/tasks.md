## 1. CLI Structure Setup

- [x] 1.1 Add `--json` flag to main `Cli` struct in `cli/src/main.rs` using `#[arg(long)] pub json: bool`
- [x] 1.2 Create `cli/src/output.rs` module with helper functions: `print_json()`, `print_json_success()`, `print_json_error()`
- [x] 1.3 Import `serde_json` in `cli/Cargo.toml` if not already present (check if `cerbo_core` exports it)

## 2. Vault Commands JSON Output

- [x] 2.1 Implement JSON output for `vault list` command - return array of vault objects with id, name, path, active fields
- [x] 2.2 Implement JSON output for `vault add` command - return vault object with id, name, path, active fields
- [x] 2.3 Implement JSON output for `vault remove` command - return `{"success": true, "message": "..."}`
- [x] 2.4 Implement JSON output for `vault active` command - return `{"success": true, "message": "..."}`

## 3. Page Commands JSON Output

- [x] 3.1 Implement JSON output for `page list` command - return array of page objects with slug, title, path fields
- [x] 3.2 Implement JSON output for `page create` command - return `{"slug": "..."}`
- [x] 3.3 Implement JSON output for `page read` command - return `{"content": "..."}`
- [x] 3.4 Implement JSON output for `page write` command - return `{"success": true, "message": "..."}`
- [x] 3.5 Implement JSON output for `page delete` command - return `{"success": true, "message": "..."}`
- [x] 3.6 Implement JSON output for `page rename` command - return `{"new_slug": "..."}`

## 4. Attachment Commands JSON Output

- [x] 4.1 Implement JSON output for `attachment list` command - return array of attachment objects with filename, path fields
- [x] 4.2 Implement JSON output for `attachment add` command - return `{"filename": "..."}`
- [x] 4.3 Implement JSON output for `attachment delete` command - return `{"success": true, "message": "..."}`

## 5. Index Commands JSON Output

- [x] 5.1 Implement JSON output for `index build` command - return `{"success": true, "message": "..."}`
- [x] 5.2 Implement JSON output for `index backlinks` command - return array of backlink objects with slug, title fields

## 6. Info and Watch Commands JSON Output

- [x] 6.1 Implement JSON output for `info` command - return object with config_dir, cache_dir, vaults array
- [x] 6.2 Implement JSON output for `watch` command - each event outputs `{"event": "...", "vault_id": "...", "message": "..."}`

## 7. Error Handling

- [x] 7.1 Wrap main logic to catch errors and output JSON error format when `--json` flag is set
- [x] 7.2 Ensure error output format: `{"error": true, "message": "...", "code": ...}`
- [x] 7.3 Ensure non-zero exit code on errors with `--json` flag

## 8. Testing

- [x] 8.1 Test all commands with `--json` flag to verify valid JSON output
- [x] 8.2 Test commands without `--json` flag to verify backward compatibility
- [x] 8.3 Test error cases with `--json` flag to verify error JSON format
- [x] 8.4 Add automated tests for JSON output validation (optional, based on existing test patterns)
