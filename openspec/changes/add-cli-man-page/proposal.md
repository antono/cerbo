## Why

The cerbo CLI tool lacks a man page, making it difficult for users to discover available commands, understand arguments, and learn about the underlying storage model. A conventional man page would improve discoverability and serve as the canonical reference for the CLI API.

## What Changes

- Add a Markdown source file for the cerbo man page (conventional format)
- Set up **mandown** (Rust crate) to convert Markdown to troff format during build
- Install the generated man page with the CLI tool (section 1)
- Document all CLI commands with explanations and argument descriptions
- Include brief description of the UUID-based storage model in the man page

## Capabilities

### New Capabilities
- `cli-man-page`: Man page for cerbo CLI with conventional format, documenting all commands, arguments, and storage model

### Modified Capabilities

<!-- No existing specs need modification -->

## Non-goals

- HTML or PDF documentation generation (man page only)
- Shell completion scripts (separate concern)
- Detailed API documentation for library usage (CLI-focused)

## Impact

- Build system: Add man page generation step in `cli` crate's `build.rs` using mandown crate
- Nix packaging: Ensure man page installs to correct `MANPATH` location (likely `share/man/man1/`)
- CLI codebase: No changes to actual CLI code, only documentation
- Dependencies: Add **mandown** crate as build dependency in `cli/Cargo.toml`
