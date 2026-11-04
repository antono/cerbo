# Development

## Environment

The project uses [Nix Flakes](https://nixos.org/explore/#nix-flakes) for every
dependency: Rust, Bun, Tauri, WebKit, `tauri-driver` and the rest.

```bash
nix develop        # enter the dev shell
nix flake show     # list packages, checks and shells
```

With [direnv](https://direnv.net/) installed, `direnv allow` loads the shell
automatically whenever you enter the directory.

Everything below runs inside that shell (or via
`nix develop --command bash -c "<command>"`).

## Day-to-day commands

```bash
bun run tauri dev          # desktop app with hot reload
bun run dev-debug          # same, with the WebKit inspector on 127.0.0.1:9222
cargo run -p cerbo -- vault list   # run the CLI from source
cargo check --workspace    # type-check all crates
bun run check              # svelte-check
```

Dependencies are managed with **bun** only (`bun add`, `bun add -d`,
`bun install`). After `package.json` or `bun.lock` changes, regenerate the Nix
manifest so `nix build` sees the same lockfile:

```bash
bunx bun2nix -o nix/bun.nix
```

## Building

Release builds go through Nix:

```bash
nix build .#cerbo            # CLI binary          -> result/bin/cerbo
nix build .#cerbo-desktop    # desktop app         -> result/bin/cerbo-desktop
nix flake check              # release-workflow and atomic-writes checks
```

The packaged desktop app ships a launcher that runs
`cerbo migrate vocabulary --if-needed` on every registered vault before the
window opens (see [storage.md](storage.md#older-vaults)).

## Tests

| Suite | Command | Notes |
|-------|---------|-------|
| Rust (core, CLI, desktop crate) | `cargo test --workspace` | core tests share temp dirs; add `-- --test-threads=1` when isolating a flaky one |
| Lints | `cargo clippy --workspace --all-targets -- -D warnings` | `core/clippy.toml` forbids raw `fs::write` outside `fsio.rs` |
| Atomic-write guard | `python3 scripts/check-atomic-writes.py core/src` | the same rule without cargo; also a flake check |
| Frontend unit | `bunx vitest run` | `src/lib/*.test.ts` |
| Desktop end-to-end | `bun run test:e2e` | see below |

### End-to-end

The e2e suite drives the built desktop app through `tauri-driver` and
WebKitWebDriver (both in the dev shell) with WebdriverIO:

```bash
nix build .#cerbo-desktop   # the app under test, at ./result
bun run test:e2e            # runs e2e/specs; uses xvfb-run when DISPLAY is unset
```

Each spec file gets temporary XDG config and cache directories; specs under
`e2e/specs/vault/` also get a fresh copy of `test-vault/`. Your own vaults and
settings are never touched. Set `CERBO_DESKTOP_BIN` to test another binary.
CI runs the suite on pushes to `main` and on pull requests
(`.github/workflows/e2e.yml`).

## Releases

`.github/workflows/release.yml` is started by hand (workflow dispatch) with a
`patch`, `minor` or `major` bump. It updates the version-bearing files, tags
the commit, builds the Linux artifacts with Nix, generates release notes and
publishes a GitHub Release.

## Repository layout

```
core/        cerbo-core: vault store, RDF metadata, semantic index, search
cli/         the `cerbo` binary; cli/man/cerbo.md is the man page source
src-tauri/   cerbo-desktop: Tauri commands, window, icons
src/         Svelte 5 frontend (Carta editor, panels, hotkeys)
e2e/         WebdriverIO end-to-end suite
test-vault/  fixture vault copied by the e2e suite
nix/         packages, dev shell, bun manifest
docs/        this documentation
openspec/    feature proposals, designs and specs (archived once shipped)
```

The CLI man page is generated at build time from `cli/man/cerbo.md`; every
change to a command or flag goes there. `cli/man/cerbo.1` is a build output and
is not committed.

## Data on disk

Configuration lives in `$XDG_CONFIG_HOME/cerbo/`, caches in
`$XDG_CACHE_HOME/cerbo/`; `cerbo info` prints both. The full layout, including
what inside a vault's `.cerbo/` directory, is in [storage.md](storage.md).
