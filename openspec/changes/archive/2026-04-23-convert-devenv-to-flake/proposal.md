## Why

The project currently uses `devenv.nix` for its development environment. Transitioning to a native `flake.nix` improves integration with the Nix ecosystem, allows for standardized installable packages, and simplifies the build process. This change enables users to build and run Cerbo (both CLI and Desktop) directly using Nix commands without requiring the `devenv` tool.

## What Changes

- **Convert** `devenv.nix` configuration into a standard `flake.nix`.
- **Implement** a `devShell` containing all necessary development tools: Rust toolchain, Bun runtime, and Tauri CLI.
- **Implement** a Nix package for `cerbo` (the Rust-based CLI).
- **Implement** a Nix package for `cerbo-desktop` (the Tauri-based desktop application).
- **Remove** `devenv.nix` and `devenv.lock` once the flake is fully functional.

## Capabilities

### New Capabilities
- `nix-tooling`: Comprehensive Nix integration including a development environment and package definitions for CLI and Desktop versions.

### Modified Capabilities
- (none)

## Impact

- **Development workflow**: Developers will now use `nix develop` to enter the environment.
- **Distribution**: Cerbo can be distributed and installed as a Nix package.
- **Build Infrastructure**: The project gains a reproducible build definition for all its components.
