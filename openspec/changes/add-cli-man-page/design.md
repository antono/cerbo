## Context

The cerbo CLI tool currently has no man page. Users must rely on `--help` output which lacks detailed explanations and storage model context. The AGENTS.md now mandates that any CLI API change must update the man page. We need a Rust-native solution to generate a conventional man page from Markdown source during the build process.

## Goals / Non-Goals

**Goals:**
- Generate a standards-compliant man page (section 1) from Markdown source
- Use a Rust-native tool (no Go, Ruby, or Python dependencies)
- Integrate seamlessly into the existing `cargo build` workflow via `build.rs`
- Document all CLI commands, arguments, and the UUID-based storage model
- Install man page to correct location in both development and Nix builds

**Non-Goals:**
- Shell completion scripts (bash/zsh/fish) - separate concern
- HTML or PDF documentation generation
- Online documentation portal
- Markdown rendering in terminal (like `mdcat`)

## Decisions

### Decision 1: Use `mandown` crate for Markdown → troff conversion

**Choice**: `mandown` (Rust crate)

**Rationale**:
- Pure Rust implementation, no external toolchain needed
- Can be used as a library directly in `build.rs`
- Generates standards-compliant groff/troff output
- Lightweight with minimal dependencies

**Alternatives considered**:
- **pulldown-cmark + custom troff emitter**: More control but requires writing and maintaining a troff emitter from scratch
- **go-md2man**: Go-based, would require Go toolchain or pre-built binary in Nix
- **pandoc**: Heavy dependency, requires Haskell toolchain

**Source**: [mandown on crates.io](https://crates.io/crates/mandown)

### Decision 2: Markdown as source format, stored in `cli/man/cerbo.md`

**Choice**: Keep man page source as Markdown in `cli/man/cerbo.md`

**Rationale**:
- Easy to read and edit (developers know Markdown)
- Can be previewed with any Markdown viewer
- Version-controlled alongside code
- `mandown` accepts Markdown input directly

**Structure of Markdown file**:
```markdown
# NAME
cerbo - Semantic knowledge management tool for agents and humans

# SYNOPSIS
`cerbo [OPTIONS] <COMMAND>`

# DESCRIPTION
...

# COMMANDS
...

# STORAGE
...
```

### Decision 3: Integrate via `build.rs` in `cli` crate

**Choice**: Add `build.rs` to `cli` crate that:
1. Reads `man/cerbo.md`
2. Converts to troff using `mandown::render_manpage()`
3. Writes to `$OUT_DIR/cerbo.1`
4. Copies to appropriate install location or sets up for Nix to install

**Rationale**:
- Standard Rust build integration
- Runs automatically on `cargo build`
- Access to cargo environment variables (`OUT_DIR`, `CARGO_MANIFEST_DIR`)

### Decision 4: Man page section 1 (user commands)

**Choice**: Install as `cerbo.1` in section 1

**Rationale**:
- Section 1 is for user commands (correct for CLI tools)
- Follows convention: `man cerbo` will find it in section 1

## Risks / Trade-offs

**[Risk] mandown output may not include all standard man macros**
→ Mitigation: Review generated troff output; mandown is designed for this purpose and produces standards-compliant output. Test with `groff -man -Tascii cerbo.1 | less`

**[Risk] build.rs adds compilation time**
→ Mitigation: mandown is lightweight; conversion is fast for a single man page. Only regenerates when `cerbo.md` changes (use `cargo:rerun-if-changed`)

**[Risk] Nix build may need special handling for man page installation**
→ Mitigation: Nix package can install pre-generated `cerbo.1` or run the build step. Check existing Nix packaging in `flake.nix`

**[Risk] Markdown source and CLI code could drift**
→ Mitigation: AGENTS.md now mandates updating man page for CLI API changes. Consider adding a test that checks CLI commands against man page (future enhancement)

## Open Questions

1. Should we pre-generate the man page and commit it to repo, or always generate during build?
   - *Recommendation*: Generate during build for accuracy, but can commit as artifact for easier inspection

2. How should the Nix package install the man page?
   - *Likely*: Use `man/` output from cargo build, install to `$out/share/man/man1/`

3. Should we include examples in the man page?
   - *Recommendation*: Yes, a simple EXAMPLES section with 2-3 common commands
