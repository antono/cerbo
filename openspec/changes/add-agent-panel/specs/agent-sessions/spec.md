# Spec Delta

## Purpose

The daemon runs agent sessions so they continue without the window and can be reopened later.

## ADDED Requirements

### Requirement: Supported agents

The daemon SHALL be able to run OpenCode and Claude Code as ACP agents, found by their command on the user's `PATH` or by a configured path. An agent that is not installed SHALL be shown as unavailable with the reason.

#### Scenario: Agent missing
- **WHEN** `opencode` is not installed
- **THEN** OpenCode SHALL be listed as unavailable and Claude Code SHALL still be usable

### Requirement: A session works on one vault

A session SHALL be tied to one vault and given that vault only, through `cerbo mcp`. The agent's own process SHALL run with the vault's folder as its working directory and with the vault's agent permission in force.

#### Scenario: Read-only session
- **WHEN** a session starts on a read-only vault
- **THEN** the agent SHALL be able to read the vault and every write SHALL be refused

### Requirement: The agent runs in a sandbox

The daemon SHALL start every agent process inside a bubblewrap sandbox in which the vault is read-only, the rest of the user's home is hidden except the agent's login file, and the main daemon socket is not reachable. The agent SHALL be able to change pages only through the session's own socket.

#### Scenario: Direct write is refused
- **WHEN** the agent tries to write a file in the vault folder with its own tools
- **THEN** the write SHALL fail and the page SHALL stay unchanged

#### Scenario: No bubblewrap
- **WHEN** `bwrap` is missing or user namespaces are disabled
- **THEN** starting a session SHALL fail with a message naming the cause

#### Scenario: Sandbox turned off
- **WHEN** the user set `agent.sandbox = "none"` in the daemon config
- **THEN** sessions SHALL start without the sandbox and each SHALL show a warning

#### Scenario: A vault that would expose the daemon's own files
- **WHEN** a session is started on a vault that is, holds or lies inside the agent homes, the daemon's config, data, runtime or socket directory, or the directory of an agent's login file, or that is or holds the user's home directory
- **THEN** starting SHALL be refused with a message naming what the vault overlaps, and no agent SHALL be started

### Requirement: A vault cannot configure its agent

The daemon SHALL start each agent so that files inside the vault cannot add tools, add MCP servers (a server named `cerbo` included), change the agent's permissions or choose its permission mode. For OpenCode the vault's `opencode.json` and `.opencode/` SHALL NOT be read. For Claude Code the session SHALL use only the MCP server the daemon passes, only the agent home's user settings, and SHALL be put into its `default` permission mode when it starts, never `bypassPermissions`.

#### Scenario: Hostile vault configuration
- **WHEN** a vault holds a `.mcp.json` with a server named `cerbo` and `.claude/settings.json` and `.claude/settings.local.json` asking for `bypassPermissions`
- **THEN** that server SHALL NOT be started and the session SHALL start in `default` mode

### Requirement: The session socket is agent-only

Every connection on a session's socket SHALL count as an agent bound to the session's vault, whatever its hello says.

#### Scenario: Agent cannot pose as the app
- **WHEN** a process in the sandbox connects to the session socket and says it is the app
- **THEN** the daemon SHALL treat it as an agent of that vault and refuse approval answers

### Requirement: Starting a session

Creating a session SHALL answer at once with the session in the `starting` state. The agent SHALL be started in the background and the session SHALL become `idle`, or `failed` with the reason, as a `state` event of its feed. A message SHALL be refused while the session is starting. A session that is ended while it starts SHALL stay ended, and its agent SHALL be stopped. A session SHALL NOT be started twice at once: a second request to resume a session that is running or starting SHALL be refused.

#### Scenario: Slow agent
- **WHEN** an agent takes a minute to start
- **THEN** the request that created the session SHALL have been answered at once, and the session SHALL become `idle` when the agent is ready

#### Scenario: Two windows resume the same session
- **WHEN** two requests to resume the same session arrive together
- **THEN** exactly one SHALL start an agent and the other SHALL be refused

#### Scenario: End during start
- **WHEN** the user ends a session while its agent is still starting
- **THEN** the session SHALL end, the agent SHALL stop, and the session SHALL NOT become `idle` afterwards

#### Scenario: Agent dies while loading
- **WHEN** the agent exits while the daemon loads its earlier session
- **THEN** the session SHALL be `failed` with how the agent ended

### Requirement: Cancel wins over a permission request

When the user cancels a turn, every permission request of that turn that is open, queued or arrives while the turn is being cancelled SHALL be answered to the agent as cancelled and SHALL NOT show a card.

#### Scenario: Request during cancel
- **WHEN** the agent asks for a permission after it was told to cancel
- **THEN** no approval card SHALL appear and the agent SHALL be answered `cancelled`

### Requirement: A client that falls behind catches up

A client whose events were dropped because it read too slowly SHALL be sent a `resync` event, and SHALL then read the session list, the feed after the last event it holds and the approvals again.

#### Scenario: Slow window
- **WHEN** an agent writes far more events than a window reads in time
- **THEN** the window SHALL receive `resync` and show the whole feed after it reads again

### Requirement: Interrupted sessions end

A stored session whose record says it was starting, idle or working while no process of the daemon runs it (the daemon stopped or crashed) SHALL get a final `state` event `ended` with the reason `daemon restarted`, once, when the daemon starts, so every client agrees on how it ended.

#### Scenario: Crash in the middle of a turn
- **WHEN** the daemon restarts after a crash during a turn
- **THEN** the session SHALL be listed as ended and its feed SHALL end with `ended`, reason `daemon restarted`

### Requirement: What an agent sends is bounded

The daemon SHALL cut a message or thought chunk, a tool call title and a tool call id to a fixed size per event, SHALL end a session with an error when its stored feed reaches 64 MiB, SHALL read only the end of a feed to resume a session, and SHALL count the approval requests an agent has waiting per session (per vault for an agent on the main socket), not per connection.

#### Scenario: Many connections
- **WHEN** an agent opens many connections on its session socket and asks for changes under `approve`
- **THEN** all of them together SHALL have at most the daemon's limit of requests waiting, and further changes SHALL be refused at once

#### Scenario: Endless output
- **WHEN** an agent writes events until the feed reaches its limit
- **THEN** the session SHALL end with an error saying so and the agent SHALL stop

### Requirement: Sessions survive the window

A session SHALL keep running when the app window closes or the app quits. Reopening the app SHALL show the running session with everything said so far.

#### Scenario: Close and return
- **WHEN** the user closes the app while the agent works and opens it again
- **THEN** the panel SHALL show the same session and its new messages

### Requirement: Session history

Each session SHALL be stored with its messages, tool calls and outcomes, listed per vault, newest first, with a title taken from its first message. The user SHALL be able to delete a session.

#### Scenario: Reopen an earlier session
- **WHEN** the user picks an earlier session from the list
- **THEN** its messages SHALL be shown and the user SHALL be able to continue it if the agent supports resuming, and otherwise start a new one with its transcript as context

### Requirement: Stop and cancel

The user SHALL be able to cancel the agent's current turn and to end a session. Ending SHALL stop the agent process. Pending approvals of the session SHALL be rejected.

#### Scenario: Cancel a turn
- **WHEN** the user cancels while the agent is working
- **THEN** the agent SHALL stop the current turn and the session SHALL stay open

### Requirement: Agent failures are reported

If the agent process exits or fails to start, the session SHALL show the reason (including the tail of its error output) and offer to start a new session. It SHALL NOT retry by itself.

#### Scenario: Agent crashes
- **WHEN** the agent process exits during a turn
- **THEN** the session SHALL show that it stopped, with the reason

### Requirement: Headless run

`cerbo agent run --vault <vault> [--agent opencode|claude] "<prompt>"` SHALL start a session, send the prompt, wait for the turn to finish, print the agent's final answer, and exit 0 on success. Under `approve`, approvals SHALL follow the daemon's approval rules (wait for any client, reject at the deadline); under `read` and `auto` it SHALL need no person. `--json` SHALL print the transcript as JSON.

#### Scenario: Interrupted command
- **WHEN** `cerbo agent run` is interrupted with `SIGINT` or `SIGTERM`, or is killed
- **THEN** the turn SHALL be cancelled where possible and the session SHALL end, and no agent SHALL be left running for it

#### Scenario: Cron job
- **WHEN** a cron job runs `cerbo agent run --vault Notes "Summarize today's inbox page"` on an `auto` vault with no app open
- **THEN** the command SHALL finish, print the summary, and any page changes SHALL be in the change log

#### Scenario: Approve mode with nobody there
- **WHEN** the same command runs on an `approve` vault and no client answers
- **THEN** the request SHALL be rejected at the deadline and the command SHALL report the rejection and exit non-zero

### Requirement: List and stop from the CLI

`cerbo agent list` SHALL show running sessions with vault, agent and state. `cerbo agent stop <session>` SHALL end one.

#### Scenario: Stop a runaway session
- **WHEN** the user runs `cerbo agent stop` for a running session
- **THEN** the session SHALL end and its agent process SHALL stop
