## 1. CLI Version Flag

- [x] 1.1 Add `version` field to `Cli` struct in `cli/src/main.rs`
- [x] 1.2 Handle version flag before subcommand parsing (print and exit)
- [x] 1.3 Test `cerbo --version` outputs version string

## 2. Desktop Version Flag

- [x] 2.1 Add version check in `src-tauri/src/main.rs` before `cerbo_lib::run()`
- [x] 2.2 Read version from `tauri.conf.json` or use compile-time constant
- [x] 2.3 Test `cerbo-desktop --version` outputs version string without launching GUI

## 3. Verification

- [x] 3.1 Run `cargo build -p cerbo && ./target/debug/cerbo --version` outputs `cerbo 0.1.0`
- [x] 3.2 Run `cargo build -p cerbo-desktop && ./target/debug/cerbo-desktop --version` outputs version string
- [x] 3.3 Run `nix build .#cerbo .#cerbo-desktop` builds both products successfully