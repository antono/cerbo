## 1. Initial Flake Setup

- [x] 1.1 Create `flake.nix` with basic structure and `devShell` mirroring `devenv.nix`.
- [x] 1.2 Verify `nix develop` provides the required tools and environment variables.

## 2. CLI Package Implementation

- [x] 2.1 Define the `cerbo` package in `flake.nix` using `buildRustPackage`.
- [x] 2.2 Verify `nix build .#cerbo` produces a functional binary.

## 3. Desktop Package Implementation

- [x] 3.1 Define a derivation for the frontend assets using `bun`.
- [x] 3.2 Define the `cerbo-desktop` package in `flake.nix` combining the frontend assets and Tauri build.
- [ ] 3.3 Verify `nix build .#cerbo-desktop` produces a functional application.

## 4. Verification and Cleanup

- [ ] 4.1 Verify that the `devShell` and both packages work as expected in a clean environment.
- [ ] 4.2 Remove `devenv.nix` and `devenv.lock`.
