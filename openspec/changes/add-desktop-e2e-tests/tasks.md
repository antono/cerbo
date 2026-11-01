# Tasks

## 1. Nix tooling

- [x] 1.1 Package `tauri-driver` 2.1.0 in `nix/pkgs.nix` and expose it in `flake.nix`; verify `nix build .#tauri-driver` produces `bin/tauri-driver`
- [x] 1.2 Add `tauri-driver`, `xvfb-run` and WebKitWebDriver to the devshell; verify `nix develop --command bash -c 'tauri-driver --help && which WebKitWebDriver xvfb-run'` succeeds

## 2. WebdriverIO harness

- [x] 2.1 Add WebdriverIO dev dependencies with `bun add -d`, regenerate `nix/bun.nix`; verify `nix build .#cerbo-frontend` still builds
- [x] 2.2 Add `e2e/wdio.conf.ts` that spawns `tauri-driver --native-driver <WebKitWebDriver>` and fails fast when the desktop binary is missing; verify running with a bogus `CERBO_DESKTOP_BIN` errors out naming the path
- [x] 2.3 Add per-session isolation (temp XDG dirs, copy of `test-vault/`, generated `vaults.toml`); verify developer `~/.config/cerbo` and `test-vault/` are unchanged after a run (`git status test-vault`, checksum of config dir)
- [x] 2.4 Add `test:e2e` script that uses `xvfb-run -a` when `DISPLAY` is unset; verify it runs with `env -u DISPLAY`

## 3. Smoke specs

- [x] 3.1 Add `e2e/specs/empty/empty-state.e2e.ts`: no vaults → "Add a vault to get started." visible; verify it passes
- [x] 3.2 Add `e2e/specs/vault/vault-open.e2e.ts`: seeded fixture vault → it is the active vault and its "Index" page is listed in the sidebar; verify it passes
- [x] 3.3 Document running e2e tests in `README.md` (build, run, headless, env overrides); verify the documented commands run as written

## 4. CI

- [x] 4.1 Add `.github/workflows/e2e.yml` (push to main, pull_request, dispatch) that builds the app with `bun run tauri build --no-bundle` and runs `bun run test:e2e` headless; verify `actionlint` passes
- [ ] 4.2 Verify the workflow passes on GitHub for a pull request

## Workflow follow-up

- Archive this change after review.
</content>
