# Proposal

## Why

The desktop app is covered only by Rust tests and Vitest unit tests of frontend
modules. Nothing drives the real `cerbo-desktop` binary, so regressions in
startup, IPC wiring, capabilities or the WebKitGTK webview reach users unnoticed.
Tauri 3 (currently alpha) brings no new e2e tooling, so the official
WebDriver route of Tauri 2 is the path that will survive the upgrade.

## What Changes

- Add an end-to-end test suite that launches the Nix-built `cerbo-desktop`
  binary through `tauri-driver` and WebKitWebDriver and drives it with
  WebdriverIO.
- Package `tauri-driver` in the flake and expose it, `xvfb-run` and the
  matching WebKitWebDriver in the devshell.
- Run every e2e session against an isolated XDG config/cache directory and a
  throwaway copy of `test-vault/`, so tests never touch the developer's vaults.
- Add a smoke suite: app starts with no vaults and shows the empty state; app
  starts with a seeded vault and lists its pages.
- Add a `bun run test:e2e` entry point that works headless under Xvfb.

## Capabilities

### New Capabilities
- `desktop-e2e-testing`: how the desktop app is exercised end to end:
  isolation from user data, headless execution, and the baseline smoke
  scenarios that must pass.

### Modified Capabilities
<!-- none -->

## Non-goals

- macOS or Windows e2e runs (no WKWebView driver on macOS; Windows not targeted).
- CI wiring in GitHub Actions; this change makes the suite runnable locally
  and headless, CI comes later.
- Broad feature coverage beyond the smoke scenarios.
- Playwright or mocked-IPC browser tests.

## Impact

- New `e2e/` directory (WebdriverIO config, specs, fixtures helper).
- `package.json`: WebdriverIO dev dependencies and `test:e2e` script;
  `nix/bun.nix` regenerated.
- `nix/pkgs.nix`, `nix/devshell.nix`, `flake.nix`: `tauri-driver` package and
  devshell tools.
- No change to app runtime code or to release builds.
</content>
