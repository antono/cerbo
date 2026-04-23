## Context

Both `cerbo` CLI and `cerbo-desktop` (Tauri app) need a `--version` flag so users can quickly check which version they're running. This is standard CLI/desktop app behavior and helps with debugging and support.

- **CLI** (`cli/`): Uses `clap` v4 with derive macros. Version is in `cli/Cargo.toml` (`version = "0.1.0"`).
- **Desktop** (`src-tauri/`): Tauri app. Version is in `src-tauri/tauri.conf.json` (`version = "0.1.0"`).

## Goals / Non-Goals

**Goals:**
- Add `--version` flag to `cerbo` CLI that prints the semver string (e.g., `0.1.0`)
- Add `--version` flag to `cerbo-desktop` CLI binary that prints the semver string
- Provide a consistent, helpful output (e.g., includes program name)

**Non-Goals:**
- No `--help` customization beyond clap defaults
- No `--verbose` or `--build-info` flags
- No runtime version fetching (versions are compile-time only)
- No changes to Tauri GUI behavior

## Decisions

### CLI: Use clap built-in `version` attribute

The `#[derive(Parser)]` macro on `Cli` struct already supports a `version` field via clap's built-in mechanism:

```rust
#[derive(Parser)]
#[command(name = "cerbo")]
#[command(about = "A local-first markdown wiki CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    #[arg(long)]
    version: bool,
}
```

Actually, clap provides `version = "..."` at the `#[command(...)]` attribute level. However, since there's no global `--version` flag on the top-level `Cli`, we handle it manually by parsing `--version` before subcommands.

**Alternative considered**: Add `version` to each subcommand. Rejected — inconsistent UX and clap already provides top-level version handling.

**Alternative considered**: Check `CARGO_PKG_VERSION` env var at build time. Not needed — clap handles this cleanly.

### Desktop: Check args before Tauri runs

For Tauri (desktop app), the binary accepts CLI args. Since `clap` isn't used in `main.rs`, we check `std::env::args()` for `--version` and print version before calling `cerbo_lib::run()`.

Version is read from `tauri.conf.json` via the Tauri app's metadata or directly from the JSON file.

## Risks / Trade-offs

| Risk | Mitigation |
|------|------------|
| Version in two places (Cargo.toml + tauri.conf.json) could drift | Both currently `0.1.0`; no automated sync needed for this change |
| Reading tauri.conf.json at runtime adds file I/O for desktop version check | Use `include_str!` at compile time in a build.rs or read once at startup |