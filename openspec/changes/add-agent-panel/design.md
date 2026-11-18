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

## Risks / Trade-offs

- [ACP and the two agents change] → keep the ACP layer small, pin protocol version in a handshake, record logs for replay tests.
- [An agent edits files directly and skips MCP] → D2: use the CLI's restriction switches, and run the agent with the vault as working directory only; document that `read` is enforced for MCP, and for direct file tools only as far as the agent CLI allows.
- [Transcripts hold private note text] → stored under the user's data directory with owner-only permissions; delete action in the UI.
- [Long feeds are heavy] → virtualize the feed and collapse finished tool rows.

## Migration Plan

New feature, no data migration. Disabled until an agent CLI is found. Rolling back removes the tab content and the `cerbo agent` commands; stored sessions can be deleted by removing the data directory.

## Open Questions

- Exact ACP spawn arguments for each agent CLI (checked against their current docs when implementing).
- Whether to show token and cost use in the header.
