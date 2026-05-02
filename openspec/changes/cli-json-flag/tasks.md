## 1. CLI Structure Setup

- [ ] 1.1 Add `--json` flag to main `Cli` struct in `cli/src/main.rs` using `#[arg(long)] pub json: bool`
- [ ] 1.2 Create `cli/src/output.rs` module with helper functions: `print_json()`, `print_json_success()`, `print_json_error()`
- [ ] 1.3 Import `serde_json` in `cli/Cargo.toml` if not already present (check if `cerbo_core` exports it)

## 2. Vault Commands JSON Output

- [ ] 2.1 Implement JSON output for `vault list` command - return array of vault objects with id, name, path, active fields
- [ ] 2.2 Implement JSON output for `vault add` command - return vault object with id, name, path, active fields
- [ ] 2.3 Implement JSON output for `vault remove` command - return `{"success": true, "message": "..."}`
- [ ] 2.4 Implement JSON output for `vault active` command - return `{"success": true, "message": "..."}`

## 3. Page Commands JSON Output

- [ ] 3.1 Implement JSON output for `page list` command - return array of page objects with slug, title, path fields
- [ ] 3.2 Implement JSON output for `page create` command - return `{"slug": "..."}`
- [ ] 3.3 Implement JSON output for `page read` command - return `{"content": "..."}`
- [ ] 3.4 Implement JSON output for `page write` command - return `{"success": true, "message": "..."}`
- [ ] 3.5 Implement JSON output for `page delete` command - return `{"success": true, "message": "..."}`
- [ ] 3.6 Implement JSON output for `page rename` command - return `{"new_slug": "..."}`

## 4. Attachment Commands JSON Output

- [ ] 4.1 Implement JSON output for `attachment list` command - return array of attachment objects with filename, path fields
- [ ] 4.2 Implement JSON output for `attachment add` command - return `{"filename": "..."}`
- [ ] 4.3 Implement JSON output for `attachment delete` command - return `{"success": true, "message": "..."}`

## 5. Index Commands JSON Output

- [ ] 5.1 Implement JSON output for `index build` command - return `{"success": true, "message": "..."}`
- [ ] 5.2 Implement JSON output for `index backlinks` command - return array of backlink objects with slug, title fields

## 6. Info and Watch Commands JSON Output

- [ ] 6.1 Implement JSON output for `info` command - return object with config_dir, cache_dir, vaults array
- [ ] 6.2 Implement JSON output for `watch` command - each event outputs `{"event": "...", "vault_id": "...", "message": "..."}`

## 7. Error Handling

- [ ] 7.1 Wrap main logic to catch errors and output JSON error format when `--json` flag is set
- [ ] 7.2 Ensure error output format: `{"error": true, "message": "...", "code": ...}`
- [ ] 7.3 Ensure non-zero exit code on errors with `--json` flag

## 8. Testing

- [ ] 8.1 Test all commands with `--json` flag to verify valid JSON output
- [ ] 8.2 Test commands without `--json` flag to verify backward compatibility
- [ ] 8.3 Test error cases with `--json` flag to verify error JSON format
- [ ] 8.4 Add automated tests for JSON output validation (optional, based on existing test patterns)
