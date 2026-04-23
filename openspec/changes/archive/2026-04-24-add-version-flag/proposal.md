## Why

Users need a way to check the installed version of both the CLI tool (`cerbo`) and the desktop application (`cerbo-desktop`). Version information is critical for debugging, support requests, and ensuring users have the correct version installed. Currently, there is no `--version` flag for either command.

## What Changes

- Add `--version` flag to the CLI tool (`cerbo`) that prints the version string
- Add `--version` flag to the desktop application (`cerbo-desktop`) that prints the version string
- If no flag is provided, the behavior remains unchanged (the app/desktop runs normally)

## Capabilities

### New Capabilities
- `cli-version-flag`: Add `--version` flag to CLI command outputting semver string
- `desktop-version-flag`: Add `--version` flag to desktop app outputting semver string

### Modified Capabilities
- None. This is a purely additive feature; existing behavior is unchanged.

## Impact

- **CLI**: Rust CLI crate (`cli/`).
- **Desktop**: Rust Tauri app (`src-tauri/`).
- Both display version from Cargo.toml / tauri.conf.json.