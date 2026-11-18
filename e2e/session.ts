// Per-session isolation: every spec file gets its own temp XDG dirs and, for
// specs under specs/vault/ and specs/editor/, its own copy of test-vault/
// (specs/editor/ also gets the generated 137 KB page from fixtures/). The app is launched
// through a wrapper script because WebKitWebDriver starts the binary itself
// and its environment is shared by all sessions.
import { execFileSync } from 'node:child_process';
import { chmodSync, cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, dirname, join, resolve, sep } from 'node:path';
import { writeLargePage } from './fixtures/large-page.ts';

const repoRoot = resolve(import.meta.dirname, '..');

export const FIXTURE_VAULT_ID = '00000000-0000-4000-8000-000000000137';
export const FIXTURE_VAULT_NAME = 'E2E Vault';
// A second vault with the same name as the fixture vault, at a path too long
// for its selector row (spec `vault-selector`: same name, long path).
export const LONG_PATH_VAULT_ID = '00000000-0000-4000-8000-000000000138';
export const LONG_PATH_VAULT_DIR = 'vault-137-long';

export function desktopBinary(): string {
  const bin = resolve(repoRoot, process.env.CERBO_DESKTOP_BIN ?? 'result/bin/cerbo-desktop');
  if (!existsSync(bin)) {
    throw new Error(`cerbo-desktop binary not found at ${bin}; run \`nix build .#cerbo-desktop\` or set CERBO_DESKTOP_BIN`);
  }
  return bin;
}

/** The first existing file among `candidates`, or undefined. */
function firstExisting(candidates: (string | undefined)[]): string | undefined {
  return candidates.find((c): c is string => !!c && existsSync(c));
}

/** Where cargo puts the binaries: `$CARGO_TARGET_DIR` or `target/`, `release` before `debug`. */
function cargoDirs(): string[] {
  const target = resolve(repoRoot, process.env.CARGO_TARGET_DIR ?? 'target');
  return [join(target, 'release'), join(target, 'debug')];
}

/**
 * The daemon the app starts: `$CERBOID_BIN`, else the one beside the desktop
 * binary, else `result-cerboid/bin/cerboid` (`nix build .#cerboid -o
 * result-cerboid`), else a cargo build.
 */
export function daemonBinary(): string {
  const found = firstExisting([
    process.env.CERBOID_BIN ? resolve(repoRoot, process.env.CERBOID_BIN) : undefined,
    join(dirname(desktopBinary()), 'cerboid'),
    join(repoRoot, 'result-cerboid', 'bin', 'cerboid'),
    ...cargoDirs().map((d) => join(d, 'cerboid')),
  ]);
  if (!found) {
    throw new Error('cerboid not found; run `nix build .#cerboid -o result-cerboid` or set CERBOID_BIN');
  }
  return found;
}

/**
 * The scripted fake ACP agent (`cerbo-fake-agent`, built by the `cerbo`
 * package: `cargo build -p cerbo --bins`). It stands in for OpenCode.
 */
export function fakeAgentBinary(): string {
  const found = firstExisting([
    process.env.CERBO_E2E_FAKE_AGENT ? resolve(repoRoot, process.env.CERBO_E2E_FAKE_AGENT) : undefined,
    join(dirname(desktopBinary()), 'cerbo-fake-agent'),
    ...cargoDirs().map((d) => join(d, 'cerbo-fake-agent')),
  ]);
  if (!found) {
    throw new Error('cerbo-fake-agent not found; run `cargo build -p cerbo --bins` or set CERBO_E2E_FAKE_AGENT');
  }
  return found;
}

/** Real bubblewrap, when `$CERBO_BWRAP` names one that can create a sandbox here. */
function usableBwrap(): string | undefined {
  const bwrap = process.env.CERBO_BWRAP;
  if (!bwrap || !existsSync(bwrap)) return undefined;
  try {
    execFileSync(bwrap, ['--unshare-all', '--ro-bind', '/', '/', 'true'], { stdio: 'ignore' });
    return bwrap;
  } catch {
    return undefined;
  }
}

export interface Session {
  root: string;
  launcher: string;
  /** Whether the agent runs in a real bubblewrap sandbox. */
  sandboxed: boolean;
}

/** The agent specs: `agent-*.e2e.ts` run with the fake agent, except `*no-agent*`. */
export const NO_AGENT_SPEC = 'no-agent';

/** The temp root of the session this worker runs (set by `createSession`). */
export function sessionRoot(): string {
  const root = process.env.CERBO_E2E_ROOT;
  if (!root) throw new Error('no e2e session: CERBO_E2E_ROOT is not set');
  return root;
}

/** Whether this session's agent runs in the real bubblewrap sandbox. */
export function sessionSandboxed(): boolean {
  return process.env.CERBO_E2E_SANDBOXED === '1';
}

/** The vault the fixture copies to `<root>/vault`. */
export function sessionVaultDir(): string {
  return join(sessionRoot(), 'vault');
}

/**
 * Tell the fake agent what to do in its next sessions: the script lives in the
 * agent's home under the daemon's data directory. Each session starts a new
 * agent process, which reads the file once at start.
 */
export function writeAgentScript(script: unknown): void {
  const home = join(sessionRoot(), 'data', 'cerbo', 'agents', 'opencode', 'home');
  mkdirSync(home, { recursive: true });
  writeFileSync(join(home, 'fake-acp-script.json'), JSON.stringify(script));
}

export function createSession(specs: string[]): Session {
  const root = mkdtempSync(join(tmpdir(), 'cerbo-e2e-'));
  const configDir = join(root, 'config');
  mkdirSync(join(configDir, 'cerbo'), { recursive: true });
  // Open every session in the wide layout (spec `desktop-responsive-layout`,
  // 1000px and up), where the specs expect docked panels and the full toolbar.
  // The desktop restores this size at start; specs for the medium and narrow
  // layouts resize the window themselves.
  writeFileSync(join(configDir, 'cerbo', 'ui.toml'), '[windowBounds]\nwidth = 1200.0\nheight = 760.0\n');

  const inDir = (dir: string) => specs.some((s) => s.includes(`${sep}specs${sep}${dir}${sep}`));
  if (inDir('vault') || inDir('editor')) {
    const vaultDir = join(root, 'vault');
    cpSync(join(repoRoot, 'test-vault'), vaultDir, { recursive: true });
    if (inDir('editor')) writeLargePage(vaultDir);
    const longVaultDir = join(root, ...Array.from({ length: 6 }, (_, i) => `nested-archive-folder-${i}`), LONG_PATH_VAULT_DIR);
    mkdirSync(longVaultDir, { recursive: true });
    // The fixture vault comes first so it stays the active one.
    const entry = (id: string, path: string) =>
      `[[vaults]]\nid = "${id}"\nname = "${FIXTURE_VAULT_NAME}"\npath = "${path}"\nisAuto = false\n`;
    writeFileSync(
      join(configDir, 'cerbo', 'vaults.toml'),
      entry(FIXTURE_VAULT_ID, vaultDir) + '\n' + entry(LONG_PATH_VAULT_ID, longVaultDir),
    );
  }

  // The daemon is this session's own: its socket lives under the session root
  // (a detached `cerboid` of the user's real session would be found otherwise),
  // it embeds with the mock embedder, and the agents are the ones set here.
  const runDir = join(root, 'run');
  mkdirSync(runDir, { mode: 0o700 });
  const daemon = join(root, 'cerboid');
  const daemonBin = daemonBinary();
  const bwrap = usableBwrap();
  const sandboxed = !!bwrap;
  const withAgent = !specs.some((s) => basename(s).includes(NO_AGENT_SPEC));
  const unavailable = '/nonexistent/cerbo-e2e';
  const agentLines = ['[agent]'];
  // Claude Code and, for the empty-state spec, OpenCode are not to be found,
  // wherever the machine running the tests keeps them.
  agentLines.push(`claude = "${unavailable}/claude-agent-acp"`);
  if (!sandboxed) agentLines.push('sandbox = "none"');
  if (withAgent) {
    agentLines.push(`opencode = "${fakeAgentBinary()}"`);
    // "Signed in": the login file the daemon checks for.
    mkdirSync(join(root, 'data', 'opencode'), { recursive: true });
    writeFileSync(join(root, 'data', 'opencode', 'auth.json'), '{}');
  } else {
    agentLines.push(`opencode = "${unavailable}/opencode"`);
  }
  // A wait for an approval ends at the deadline; a spec that needs the
  // timeout shortens it itself.
  writeFileSync(join(configDir, 'cerbo', 'agents.toml'), agentLines.join('\n') + '\n');
  writeFileSync(
    daemon,
    [
      '#!/bin/sh',
      "export CERBO_TEST_MOCK_EMBEDDER=1",
      ...(bwrap ? [`export CERBO_BWRAP='${bwrap}'`] : []),
      // A `cerbo` beside a cargo-built daemon is the agents' MCP shim; the
      // Nix package of the daemon knows its own.
      ...(existsSync(join(dirname(daemonBin), 'cerbo')) ? [`export CERBO_BIN='${join(dirname(daemonBin), 'cerbo')}'`] : []),
      `exec '${daemonBin}' "$@"`,
      '',
    ].join('\n'),
  );
  chmodSync(daemon, 0o755);

  const launcher = join(root, 'cerbo-desktop');
  writeFileSync(
    launcher,
    [
      '#!/bin/sh',
      `export XDG_CONFIG_HOME='${configDir}'`,
      `export XDG_CACHE_HOME='${join(root, 'cache')}'`,
      `export XDG_DATA_HOME='${join(root, 'data')}'`,
      `export XDG_RUNTIME_DIR='${runDir}'`,
      `export CERBOID_BIN='${daemon}'`,
      `exec '${desktopBinary()}' "$@"`,
      '',
    ].join('\n'),
  );
  chmodSync(launcher, 0o755);
  // The worker runs the specs of this session: they find the temp root here.
  process.env.CERBO_E2E_ROOT = root;
  process.env.CERBO_E2E_SANDBOXED = sandboxed ? '1' : '';
  return { root, launcher, sandboxed };
}

/** The daemon of a session outlives the app (it runs on its own): stop it. */
function stopDaemon(root: string): void {
  let pid: number | undefined;
  try {
    pid = Number.parseInt(readFileSync(join(root, 'run', 'cerbo', 'cerboid.lock'), 'utf8').trim(), 10);
  } catch {
    return;
  }
  if (!pid || pid <= 1) return;
  try {
    process.kill(pid, 'SIGTERM');
  } catch {
    return;
  }
  const deadline = Date.now() + 5_000;
  while (Date.now() < deadline) {
    try {
      process.kill(pid, 0);
    } catch {
      return;
    }
    Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 50);
  }
  try {
    process.kill(pid, 'SIGKILL');
  } catch {
    // Gone in the meantime.
  }
}

export function destroySession(root: string): void {
  stopDaemon(root);
  rmSync(root, { recursive: true, force: true });
}
