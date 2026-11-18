# Design

## Context

`add-cerboid-daemon` gives one owner of the vault, the `cerbo mcp` tools and per-vault permissions. `add-right-panel-tabs` gives the tab host and an Agent placeholder. This change adds the agent itself. See proposal.md for motivation.

## Goals / Non-Goals

**Goals:**
- Chat with OpenCode or Claude Code from the Agent tab.
- Sessions that keep running without the window, and a headless `cerbo agent run`.
- All vault access of the agent goes through `cerbo mcp`, so permissions apply.

**Non-Goals:**
- Shipping or managing agent CLIs, or handling their API keys or login.
- Letting the agent touch files outside the vault.
- Multi-agent orchestration or several agents in one session.

## Decisions

**D1. The daemon is the ACP client, the window is a viewer.** The daemon spawns the agent over stdio and speaks ACP to it; the window subscribes to the session's events through the daemon socket. That is what lets a session outlive the window. Alternative: ACP client in the Tauri process. Rejected: sessions would die with the app and headless runs would need a second implementation.

**D2. Vault access only through MCP.** At session start the daemon passes the agent an MCP server entry `cerbo mcp --vault <v>`. The agent's own file tools are not relied on for the vault: the permission check lives in the daemon, so an agent that edits files directly would bypass it. Where an agent CLI offers a switch to restrict its file tools to read-only or off, the daemon uses it.

**D3. Agent permission requests are mapped, not duplicated.** ACP agents ask the client for permission to run their own tools. Those requests become cards like vault approvals but are answered under the same mode rules: `read` and `approve` ask, `auto` allows tools that only touch the vault and still asks for anything else (shell, network). Vault writes keep going through the daemon's approval queue.

**D4. Session store.** Sessions are saved as append-only event logs per vault under the daemon's data directory, outside the vault, so transcripts never end up in the user's notes or sync. The panel's feed is rendered from the log, so reopening and live streaming use one code path.

**D5. One feed model.** Events: user message, agent text chunk, tool call (start, update, end), approval request and outcome, page-changed, error. The UI is a pure function of the event list, which makes it testable without an agent: tests replay recorded logs.

**D6. Resuming.** If the agent supports resuming an ACP session, continue it; otherwise start a new one and give it the stored transcript summary as context. The UI shows which happened.

**D7. Headless run is a thin CLI over the same session API.** `cerbo agent run` creates a session, sends the prompt, waits for the turn to end and prints the answer. It never starts its own agent process.

**D8. The agent runs in a bubblewrap sandbox.** The daemon never starts an agent process directly; it starts `bwrap` with the agent as the command. The sandbox has the host network (the agent needs its model API), `/nix`, `/etc` (and `/.host-etc` on NixOS, where `resolv.conf` and certificates point), `/run/current-system`, a private `/tmp`, `/dev` and `/proc`, a tmpfs over `/home` and `/run`, the vault bound **read-only** as the working directory, the agent's private home (D10) read-write, and the session socket (D9). Nothing else of the user's home and not the main daemon socket. Flags: `--unshare-all --share-net --die-with-parent --new-session --clearenv`, with only the variables the agent needs. The `bwrap` path is fixed at build time (`CERBO_BWRAP`, set by the Nix wrapper), not looked up on `PATH`. If `bwrap` is missing or a probe sandbox fails (for example user namespaces are disabled), the session does not start and says why. Running without a sandbox needs an explicit `agent.sandbox = "none"` in the daemon config and shows a warning on every session. Alternatives: landlock (needs a newer kernel and does not hide paths), no sandbox (the permission model would be advisory). Rejected.

**D9. A socket per session.** The daemon listens on `$XDG_RUNTIME_DIR/cerbo/sessions/<id>/agent.sock` for each session and binds it into the sandbox. Every connection on it is an agent connection bound to the session's vault, whatever its hello says; app and CLI kinds, admin lines and approval answers are refused there. `cerbo mcp` takes the socket from `CERBO_SOCKET` (or `--socket`). The socket is removed when the session ends. This closes the gap where an agent could connect to the main socket and claim to be the app.

**D10. Agent homes owned by the daemon.** Each agent kind gets a home under the daemon's data directory (`agents/<agent>/home`), bound as `HOME` with `XDG_*` inside it, so the user's global agent config (MCP servers, skills, plugins, project trust) is not loaded. The daemon writes the agent's config there at each start: built-in edit, write and shell tools denied, the `cerbo` MCP tools allowed, project config in the vault ignored (`OPENCODE_DISABLE_PROJECT_CONFIG=1` for OpenCode). Only the login is shared from the real home, as single-file read-write binds: `~/.local/share/opencode/auth.json` for OpenCode, `~/.claude/.credentials.json` for Claude Code. If the login file is absent, the session reports that the agent is not logged in and how to log in.

**D11. Agents and commands.** OpenCode: `opencode acp`. Claude Code: the ACP adapter `claude-agent-acp` (npm `@agentclientprotocol/claude-agent-acp`, nixpkgs `claude-agent-acp`), which runs the Claude Agent SDK. Both are found on `PATH` when the daemon starts or at a configured absolute path, resolved to a real path, and that path is bound read-only if it lies outside `/nix`. The ACP client is hand-written (initialize, session/new, session/load, session/prompt, session/cancel, session/update, session/request_permission) rather than the `agent-client-protocol` crate, which had four major versions in a month. It advertises no `fs` and no `terminal` capability and answers any such request with method-not-found. Protocol version 1 is pinned.

## Risks / Trade-offs

- [The sandbox shares the network] → the agent can reach services on localhost and the internet; the sandbox protects files and the daemon socket, not the network. Documented in the man page and the panel.
- [The login file is readable by the agent] → it has to be, for the agent to authenticate; it is the only file of the real home in the sandbox.

- [ACP and the two agents change] → keep the ACP layer small, pin protocol version in a handshake, record logs for replay tests.
- [An agent edits files directly and skips MCP] → D2: use the CLI's restriction switches, and run the agent with the vault as working directory only; document that `read` is enforced for MCP, and for direct file tools only as far as the agent CLI allows.
- [Transcripts hold private note text] → stored under the user's data directory with owner-only permissions; delete action in the UI.
- [Long feeds are heavy] → virtualize the feed and collapse finished tool rows.

## Migration Plan

New feature, no data migration. Disabled until an agent CLI is found. Rolling back removes the tab content and the `cerbo agent` commands; stored sessions can be deleted by removing the data directory.

## Open Questions

- Claude Code's built-in tools: resolved from the adapter source (0.106). The agent home's `.claude/settings.json` denies them (`permissions.deny`) and `session/new` carries `_meta.claudeCode.options` with `tools: []`, `disallowedTools` and `settingSources: ["user"]` (see `sandbox::claude_session_meta`). The session manager passes that meta in `session/new` and `session/load`; still to verify against a real adapter.
- Whether to show token and cost use in the header.
