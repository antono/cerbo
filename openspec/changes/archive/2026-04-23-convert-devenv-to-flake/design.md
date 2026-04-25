## Context

The project currently relies on `devenv`, which is a wrapper around Nix flakes. Moving to a native `flake.nix` provides better control and allows the project to be used as a flake input by other projects.

## Goals / Non-Goals

**Goals:**
- Provide a `devShell` with all necessary tools (Rust, Bun, Tauri CLI).
- Provide an installable package for the `cerbo` CLI.
- Provide an installable package for the `cerbo-desktop` application.
- Ensure reproducible builds for both components.

**Non-Goals:**
- Supporting non-Linux platforms in the initial flake (focusing on the user's current environment).
- Implementing complex CI/CD pipelines within the flake itself.

## Decisions

### 1. Build Tooling
**Decision**: Use `nixpkgs` standard `buildRustPackage` and `buildNpmPackage` (or equivalent for Bun).
**Rationale**: Minimizes external dependencies and uses standard Nix patterns.
**Alternatives**: `crane` was considered for its advanced caching, but `buildRustPackage` is simpler for the initial transition.

### 2. Monorepo Handling
**Decision**: Use `cargo` workspace filtering or separate package definitions for `cli/` and `src-tauri/`.
**Rationale**: Allows building individual components without building the entire workspace if not needed.

### 3. Tauri Desktop Build
**Decision**: Use a two-stage build process where the frontend is built first, then passed to the Tauri Rust build.
**Rationale**: This is the standard way to build Tauri apps with Nix.

## Risks / Trade-offs

- **[Risk]** Tauri system dependencies might differ across distributions.
  - **Mitigation**: Use `pkg-config` and explicitly list all required libraries from `nixpkgs`.
- **[Risk]** Bun build might require network access if not handled correctly with a lockfile.
  - **Mitigation**: Use `bun.lock` (if available) or `import-npm-lock` patterns to ensure reproducibility.

## Migration Plan

1. Create `flake.nix`.
2. Verify `nix develop` provides the correct environment.
3. Implement and verify `nix build .#cerbo`.
4. Implement and verify `nix build .#cerbo-desktop`.
5. Remove `devenv.nix` and `devenv.lock`.
