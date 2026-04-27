## 1. Add xdg crate dependency

- [x] 1.1 Add `xdg` crate to `core/Cargo.toml`
- [x] 1.2 Remove `directories` crate from `cli/Cargo.toml` (if only used for paths)

## 2. Create CoreContext in core crate

- [x] 2.1 Create `core/src/context.rs` with `CoreContext` struct
- [x] 2.2 Implement `CoreContext::new()` using `xdg::BaseDirectories`
- [x] 2.3 Export `CoreContext` from `core/src/lib.rs`

## 3. Update CLI to use CoreContext

- [x] 3.1 Update `cli/src/main.rs` to use `CoreContext`
- [x] 3.2 Add `info` command to CLI

## 4. Update Desktop to use CoreContext

- [x] 4.1 Update `src-tauri/src/lib.rs` to use `CoreContext`
- [x] 4.2 Add `--info` flag handling

## 5. Implement info output

- [x] 5.1 Add `page_count()` to vault module
- [x] 5.2 Implement `info` command output in CLI
- [x] 5.3 Implement `--info` output in desktop