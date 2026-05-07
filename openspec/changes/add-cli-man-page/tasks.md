## 1. Setup

- [ ] 1.1 Add `mandown` crate to `cli/Cargo.toml` as build dependency
- [ ] 1.2 Create `cli/man/` directory for Markdown source
- [ ] 1.3 Verify mandown is available: `cargo build -p cerbo-cli` (should compile with new dep)

## 2. Create Man Page Source

- [ ] 2.1 Create `cli/man/cerbo.md` with NAME section (cerbo - Semantic knowledge management tool)
- [ ] 2.2 Add SYNOPSIS section with command syntax
- [ ] 2.3 Add DESCRIPTION section explaining cerbo's purpose
- [ ] 2.4 Add COMMANDS section documenting: vault (list, add, remove), init, help
- [ ] 2.5 Add OPTIONS section with global and per-command arguments
- [ ] 2.6 Add STORAGE section describing UUID-based storage model (.cerbo/ directory, Objects, Pages, Links, Annotations)
- [ ] 2.7 Add EXAMPLES section with 2-3 common commands
- [ ] 2.8 Add SEE ALSO section referencing related tools/documentation

## 3. Build Integration

- [ ] 3.1 Create/modify `cli/build.rs` to read `man/cerbo.md`
- [ ] 3.2 Add mandown conversion logic using `mandown::render_manpage()`
- [ ] 3.3 Write troff output to `$OUT_DIR/cerbo.1`
- [ ] 3.4 Add `println!("cargo:rerun-if-changed=man/cerbo.md");` to avoid unnecessary rebuilds
- [ ] 3.5 Test build: `cargo build -p cerbo-cli` and verify `cerbo.1` generated in target dir

## 4. Nix Packaging

- [ ] 4.1 Update `flake.nix` to install man page from build output to `$out/share/man/man1/cerbo.1`
- [ ] 4.2 Test Nix build: `nix build .#cerbo` and verify man page in result
- [ ] 4.3 Verify `man -M result/share/man cerbo` displays the man page correctly

## 5. Verification

- [ ] 5.1 Verify all CLI commands from `src/cli/commands/` are documented in man page
- [ ] 5.2 Cross-check command arguments with actual clap definitions
- [ ] 5.3 Test man page rendering: `groff -man -Tascii cerbo.1 | less` (check formatting)
- [ ] 5.4 Run existing tests: `cargo test -p cerbo-cli` to ensure no regressions
- [ ] 5.5 Update AGENTS.md if needed (already done - CLI API changes mandate man page updates)
