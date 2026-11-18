## Why

With `cerboid` owning the vault, an agent can safely work on it. The user wants to talk to a local coding agent (OpenCode or Claude Code) from inside Cerbo, and also run one with the app closed.

## What Changes

- The daemon starts a local agent CLI (OpenCode or Claude Code) over ACP, the Agent Client Protocol, and keeps the session alive after the window closes. Cerbo hands the agent the vault through `cerbo mcp`.
- The Agent tab of the right panel becomes a chat: message feed, tool-call rows, approval cards with a diff, a composer with `@` page mentions and the open page as context.
- Pick the agent (OpenCode or Claude Code), start a new session, and reopen earlier sessions of the vault.
- Show the vault's permission (read-only, auto, approve) in the panel and let the user change it.
- Headless use: `cerbo agent run --vault <name> "<prompt>"` runs a session without the app and prints the result.

## Capabilities

### New Capabilities
- `agent-sessions`: starting, running, stopping and storing ACP sessions in the daemon, including headless runs.
- `desktop-agent-panel`: the chat UI in the right panel's Agent tab.

### Modified Capabilities
<!-- none -->

## Impact

- `cerboid`: ACP client, session store, agent process supervision.
- `core`: RPC types for sessions and events.
- `cli`: `cerbo agent run|list|stop`; man page entry.
- Frontend: Agent tab component and its stores; builds on `add-right-panel-tabs` (tab host) and `add-cerboid-daemon` (daemon, MCP, permissions). Both must land first.
- Requires an agent CLI installed by the user; Cerbo does not ship one and never sees its API keys.
