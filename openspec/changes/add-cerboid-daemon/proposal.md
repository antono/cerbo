## Why

Today the desktop app, the `cerbo` CLI and any script each open the vault and its index on their own, held apart only by file locks. An agent that edits pages for you, including when the app is closed (cron, terminal, another editor), needs one owner of the vault: one write queue, one index, one place to ask permission.

## What Changes

- Add `cerboid`, one daemon per user that owns every registered vault: reads, writes, index, SPARQL, semantic search and file watching all go through it.
- The daemon listens on a local socket that only the user can reach. The desktop app, the CLI and agents are its clients.
- Add `cerbo mcp`: a stdio MCP server that agent CLIs (OpenCode, Claude Code) start. It forwards each tool call to the daemon.
- Add per-vault agent permissions: **read-only**, or **manage** with mode **auto** (agent edits apply at once) or **approve** (each edit waits for a person).
- Add a Home Manager module that installs `cerboid` and runs it as a systemd user service.
- Desktop becomes a client of the daemon, starting it when it is not running and not managed by systemd.
- `cerbo daemon` commands: `start`, `stop`, `status`, `logs path`.
- CLI commands keep working: they use the daemon when it runs and open the vault directly when it does not.

## Capabilities

### New Capabilities
- `cerboid-daemon`: lifecycle, socket, single instance, vault ownership, event stream for clients.
- `cerbo-mcp`: the MCP tools an agent sees, and how they map to vault operations.
- `agent-vault-permissions`: read-only, auto and approve levels, approval requests, headless behaviour.
- `home-manager-module`: the Nix flake's Home Manager module and the systemd user service.
- `desktop-daemon-client`: how the desktop finds, starts and uses the daemon.

### Modified Capabilities
- `application-lifecycle`: the app connects to the daemon before opening a vault.
- `cli-man-page`: the man page documents `cerbo daemon` and `cerbo mcp`.

## Impact

- New crate `cerboid` in the Cargo workspace; shared RPC types in `core`.
- `cli`: new `daemon` and `mcp` subcommands; existing commands gain a daemon-first path; `cli/man/cerbo.md` updated.
- `src-tauri`: vault operations move behind a daemon client, in stages.
- `flake.nix`, `nix/`: new package output and `homeManagerModules.default`.
- `core/src/app_lock.rs`: the daemon takes the same lock a running app takes, so automatic vault rewrites stay skipped while it runs.
- Builds on `add-right-panel-tabs` only for the Agent tab; `add-agent-panel` depends on this change.
