# Design

## Context

The vault is plain files plus an index (`core/src/semantic`, oxigraph). Consistency between processes rests on two advisory `flock`s: `app_lock.rs` (a running app holds it shared; automatic rewrites take it exclusively and skip when they cannot) and `semantic/lock.rs` (one process holds the index per operation, others retry with backoff). That is enough for occasional CLI use. It is not enough for an agent making many edits beside the app. See proposal.md for motivation.

## Goals / Non-Goals

**Goals:**
- One owner of every vault per user; app, CLI and agents are clients.
- The agent works with the GUI closed.
- Permissions that are simple to explain and enforced in one place.
- A service that Nix users get from one option.

**Non-Goals:**
- Network access or multi-user serving. The socket is local and owner-only.
- A sync or backup feature.
- Per-page or per-folder permissions. Permission is per vault.
- The ACP chat itself (`add-agent-panel`).

## Decisions

**D1. A new crate `cerboid`, logic stays in `core`.** The daemon is a thin server over `cerbo-core`: it hosts vault handles, the write queue and the index. RPC request and response types live in `core` so the CLI, app and MCP shim share them. Alternative: put the server in the `cli` crate. Rejected: the app would depend on the CLI crate.

**D2. One user-wide process, one socket.** Socket at `$XDG_RUNTIME_DIR/cerbo/cerboid.sock`, mode 0600 in a 0700 directory. This matches the one-per-user `desktop.lock`. Single-instance is enforced by a lock file beside the socket, not by the socket's existence, so a stale socket is simply replaced.

**D3. RPC: newline-delimited JSON over the socket.** Requests with ids, responses, and a separate subscription for events (page changed, approval requested). Human-readable, easy to test with `socat`, and the project already depends on `serde_json`. Alternative: gRPC or a binary protocol. Rejected: heavy for local use.

**D4. Write queue per vault.** All writes of a vault run on one task in arrival order, using the existing durable-write helpers. Reads do not queue. The daemon takes the app lock shared for its lifetime, so automatic migrations keep skipping.

**D5. `cerbo mcp` is a shim.** It speaks MCP on stdio and forwards to the daemon with the vault fixed at start. It carries no vault logic, so there is a single place where permission is checked: the daemon. The shim identifies itself as an agent connection and names its one vault in the hello; the daemon binds the connection to that vault. App and CLI connections are not subject to agent permissions. This is a guard for agents that work through `cerbo mcp`, not a security boundary: any process of the same user can open the socket and declare itself the app or the CLI. Real isolation of agent processes is planned with the agent panel (a design decision is pending).

**D6. Permission model.** Stored per vault as `read | auto | approve`, in the daemon's config next to `vaults.toml`, default `read`. The daemon checks it on every agent write request, so a change applies to connected agents at once. Write tools are not even listed at `read`. No tool reads or changes the setting. A write that waits for approval applies to the page as it was when requested: if the page changed meanwhile, the approval ends in a conflict instead of overwriting the edit. A request whose agent disconnects is cancelled. One agent connection may have 137 requests waiting.

**D7. Approvals live in the daemon.** A pending approval holds the diff and a deadline (default 10 minutes, configurable). Any connected client may answer; a client connecting later sees the queue. At the deadline the request is rejected and the agent is told it timed out. Alternative: block the agent forever. Rejected: a forgotten request would hang a cron job.

**D8. Delete is trash.** Agent deletes use the existing trash (`object-trash`), so even `auto` cannot destroy a page irreversibly. Every applied agent change is appended to a change log, which also serves as the audit trail for `auto`.

**D9. Start-up paths.** Three, in order: systemd socket activation (Home Manager module), the app or `cerbo mcp` spawning `cerboid` itself, `cerbo daemon start`. The client tries to connect first and only then starts a process, so no path can create two daemons (D2).

**D10. Home Manager module.** `homeManagerModules.default` in the flake adds `services.cerbo.daemon.{enable,package,vaults.<name>.agent}`. Enabling installs `cerboid` and `cerbo` and defines `systemd.user.services.cerboid` (restart on failure) and `systemd.user.sockets.cerboid` (activation). Vault permissions from Nix are written when the service starts.

**D11. Staged desktop migration.** First `cerboid` runs beside the app and only serves the CLI and MCP. Then the app's Tauri commands are moved behind a daemon client one group at a time (pages, attachments, links, index, search). The old direct path is deleted when the last group moves, not kept as a fallback, because a fallback is exactly the second writer this change removes.

**D12. CLI is daemon-first.** Existing `cerbo` commands try the daemon and fall back to opening the vault directly when no daemon runs, as today. This keeps scripts working on machines without the service.

## Risks / Trade-offs

- [A long-running process holds the vault and index in memory] → bounded caches, `cerbo daemon status` shows memory use, systemd restarts on failure.
- [Version mismatch after an upgrade leaves an old daemon] → protocol version handshake (clients refuse and tell the user), and the Nix switch restarts the service.
- [`auto` lets an agent rewrite many pages] → trash instead of delete, change log, and `read` as the default.
- [Staged migration leaves two paths for a while] → in that period the app holds the shared app lock and the daemon serves only agent and CLI traffic, so the existing locks still protect the files; ordering of tasks keeps the window short.
- [Approvals unanswered while headless] → deadline rejects them, so jobs end instead of hanging.

## Migration Plan

1. Ship `cerboid`, `cerbo daemon`, `cerbo mcp` and the permissions; the app is unchanged.
2. Ship the Home Manager module.
3. Move the app to the daemon client in stages; remove the direct path last.

Rollback: stop the service and remove the option. Vault files are never in a daemon-only format, so the CLI and older app builds keep working.

## Open Questions

- Default approval deadline (10 minutes) may need tuning after real use.
- Whether the change log is also shown in the app, or only readable through the CLI first.
