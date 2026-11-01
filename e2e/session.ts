// Per-session isolation: every spec file gets its own temp XDG dirs and, for
// specs under specs/vault/, its own copy of test-vault/. The app is launched
// through a wrapper script because WebKitWebDriver starts the binary itself
// and its environment is shared by all sessions.
import { chmodSync, cpSync, existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve, sep } from 'node:path';

const repoRoot = resolve(import.meta.dirname, '..');

export const FIXTURE_VAULT_ID = '00000000-0000-4000-8000-000000000137';
export const FIXTURE_VAULT_NAME = 'E2E Vault';

export function desktopBinary(): string {
  const bin = resolve(repoRoot, process.env.CERBO_DESKTOP_BIN ?? 'result/bin/cerbo-desktop');
  if (!existsSync(bin)) {
    throw new Error(`cerbo-desktop binary not found at ${bin}; run \`nix build .#cerbo-desktop\` or set CERBO_DESKTOP_BIN`);
  }
  return bin;
}

export interface Session {
  root: string;
  launcher: string;
}

export function createSession(specs: string[]): Session {
  const root = mkdtempSync(join(tmpdir(), 'cerbo-e2e-'));
  const configDir = join(root, 'config');
  mkdirSync(join(configDir, 'cerbo'), { recursive: true });

  if (specs.some((s) => s.includes(`${sep}specs${sep}vault${sep}`))) {
    const vaultDir = join(root, 'vault');
    cpSync(join(repoRoot, 'test-vault'), vaultDir, { recursive: true });
    writeFileSync(
      join(configDir, 'cerbo', 'vaults.toml'),
      `[[vaults]]\nid = "${FIXTURE_VAULT_ID}"\nname = "${FIXTURE_VAULT_NAME}"\npath = "${vaultDir}"\nisAuto = false\n`,
    );
  }

  const launcher = join(root, 'cerbo-desktop');
  writeFileSync(
    launcher,
    [
      '#!/bin/sh',
      `export XDG_CONFIG_HOME='${configDir}'`,
      `export XDG_CACHE_HOME='${join(root, 'cache')}'`,
      `export XDG_DATA_HOME='${join(root, 'data')}'`,
      `exec '${desktopBinary()}' "$@"`,
      '',
    ].join('\n'),
  );
  chmodSync(launcher, 0o755);
  return { root, launcher };
}

export function destroySession(root: string): void {
  rmSync(root, { recursive: true, force: true });
}
